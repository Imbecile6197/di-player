use di_player::api::{Channel, ChannelFilter, DiClient};
use di_player::config::{self, AppSettings, Credentials};
use di_player::cover_fx::{self, AccentColor};
use di_player::i18n;
use di_player::icons::APP_ICON_NAME;
use di_player::player::{MpvPlayer, PlaybackState};
use di_player::quality::StreamQuality;
use di_player::theme::{AppLocale, ChannelViewMode, UiTheme};
use di_player::tray::{self, DiTray, TrayCommand};
use di_player::visualizer::{VizStyle, Visualizer};
use gdk4::Texture;
use glib::{clone, timeout_add_local, timeout_add_seconds_local, ControlFlow};
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, ContentFit, FlowBox, FlowBoxChild, Image, Label, ListBox,
    ListBoxRow, Orientation, Overlay, Picture, PolicyType, Scale, ScrolledWindow, SearchEntry,
    SelectionMode, Stack, StackTransitionType, StringList, ToggleButton,
};
use ksni::blocking::Handle as TrayHandle;
use libadwaita as adw;
use libadwaita::prelude::*;
use parking_lot::Mutex;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

struct RuntimeState {
    channels: Vec<Channel>,
    filters: Vec<ChannelFilter>,
    playlists: HashMap<String, String>,
    credentials: Option<Credentials>,
    current: Option<Channel>,
    track_label: String,
    active_filter: Option<u64>,
    show_favorites_only: bool,
    now_playing_mode: bool,
}

#[derive(Clone)]
struct Shared {
    window: adw::ApplicationWindow,
    client: Arc<DiClient>,
    player: Arc<MpvPlayer>,
    state: Rc<RefCell<RuntimeState>>,
    settings: Rc<RefCell<AppSettings>>,
    tray_handle: Arc<Mutex<Option<TrayHandle<DiTray>>>>,
    toast: adw::ToastOverlay,
    list: ListBox,
    grid: FlowBox,
    channels_stack: Stack,
    filter_box: GtkBox,
    search: SearchEntry,
    channel_label: Label,
    track_label: Label,
    artist_label: Label,
    np_channel_label: Label,
    np_track_label: Label,
    np_artist_label: Label,
    cover: Image,
    np_cover: Image,
    backdrop: Picture,
    play_btn: Button,
    np_play_btn: Button,
    mute_btn: Button,
    status_dot: GtkBox,
    account_label: Label,
    quality_label: Label,
    visualizer: Visualizer,
    np_visualizer: Visualizer,
    content_stack: Stack,
    header_title: adw::WindowTitle,
    login_btn: Button,
    settings_btn: Button,
    refresh_btn: Button,
    fullscreen_btn: Button,
    shortcuts_btn: Button,
    view_list_btn: ToggleButton,
    view_grid_btn: ToggleButton,
    fav_tab_btn: ToggleButton,
    all_tab_btn: ToggleButton,
    brand_label: Label,
    accent_label: Label,
}

pub struct DiPlayerWindow {
    pub window: adw::ApplicationWindow,
}

impl DiPlayerWindow {
    pub fn new(app: &adw::Application) -> Self {
        let settings = Rc::new(RefCell::new(AppSettings::load()));
        i18n::set_locale(settings.borrow().app_locale());

        let client = Arc::new(DiClient::new().expect("HTTP client"));
        let player = Arc::new(MpvPlayer::new());
        let _ = player.set_volume(if settings.borrow().muted {
            0.0
        } else {
            settings.borrow().volume
        });

        let state = Rc::new(RefCell::new(RuntimeState {
            channels: Vec::new(),
            filters: Vec::new(),
            playlists: HashMap::new(),
            credentials: config::load_credentials(),
            current: None,
            track_label: String::new(),
            active_filter: None,
            show_favorites_only: false,
            now_playing_mode: false,
        }));

        let (tray_tx, tray_rx) = mpsc::channel::<TrayCommand>();
        let tray_rx = Rc::new(RefCell::new(tray_rx));
        let tray_handle: Arc<Mutex<Option<TrayHandle<DiTray>>>> = Arc::new(Mutex::new(None));
        {
            let tray_handle = tray_handle.clone();
            thread::spawn(move || {
                if let Ok(handle) = tray::spawn_service(tray_tx) {
                    *tray_handle.lock() = Some(handle);
                } else {
                    eprintln!("DI Player: StatusNotifierItem tray unavailable");
                }
            });
        }

        let initial_w = settings.borrow().window_width.clamp(640, 4096);
        let initial_h = settings.borrow().window_height.clamp(480, 4096);
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(&i18n::t("app-title"))
            .default_width(initial_w)
            .default_height(initial_h)
            .resizable(true)
            .icon_name(APP_ICON_NAME)
            .build();
        window.set_size_request(560, 420);
        window.add_css_class("di-player");
        apply_theme(&window, settings.borrow().ui_theme());

        let toast = adw::ToastOverlay::new();
        let toolbar = adw::ToolbarView::new();

        let header = adw::HeaderBar::new();
        let header_title =
            adw::WindowTitle::new(&i18n::t("app-title"), &i18n::t("app-subtitle"));
        header.set_title_widget(Some(&header_title));

        let login_btn = Button::from_icon_name("avatar-default-symbolic");
        let settings_btn = Button::from_icon_name("emblem-system-symbolic");
        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        let fullscreen_btn = Button::from_icon_name("view-fullscreen-symbolic");
        let shortcuts_btn = Button::from_icon_name("input-keyboard-symbolic");
        header.pack_end(&login_btn);
        header.pack_end(&settings_btn);
        header.pack_end(&shortcuts_btn);
        header.pack_end(&fullscreen_btn);
        header.pack_end(&refresh_btn);
        toolbar.add_top_bar(&header);

        // Backdrop layer
        let overlay = Overlay::new();
        let backdrop = Picture::new();
        backdrop.set_content_fit(ContentFit::Cover);
        backdrop.add_css_class("di-backdrop");
        backdrop.set_hexpand(true);
        backdrop.set_vexpand(true);
        let backdrop_dim = GtkBox::new(Orientation::Vertical, 0);
        backdrop_dim.add_css_class("di-backdrop-overlay");
        backdrop_dim.set_hexpand(true);
        backdrop_dim.set_vexpand(true);
        let backdrop_stack = Overlay::new();
        backdrop_stack.set_child(Some(&backdrop));
        backdrop_stack.add_overlay(&backdrop_dim);

        let hero = build_hero();
        let visualizer = Visualizer::new();
        visualizer.set_style(settings.borrow().viz_style());
        visualizer.set_enabled(settings.borrow().viz_enabled);

        let search = SearchEntry::new();
        search.set_hexpand(true);
        search.set_margin_start(12);
        search.set_margin_end(12);
        search.set_margin_top(8);

        let tab_row = GtkBox::new(Orientation::Horizontal, 8);
        tab_row.set_margin_start(12);
        tab_row.set_margin_end(12);
        tab_row.set_margin_top(6);
        let all_tab_btn = ToggleButton::with_label(&i18n::t("tab-channels"));
        let fav_tab_btn = ToggleButton::with_label(&i18n::t("tab-favorites"));
        all_tab_btn.set_active(true);
        fav_tab_btn.set_group(Some(&all_tab_btn));
        let view_list_btn = ToggleButton::new();
        view_list_btn.set_icon_name("view-list-symbolic");
        let view_grid_btn = ToggleButton::new();
        view_grid_btn.set_icon_name("view-grid-symbolic");
        view_grid_btn.set_group(Some(&view_list_btn));
        match settings.borrow().channel_view_mode() {
            ChannelViewMode::List => view_list_btn.set_active(true),
            ChannelViewMode::Grid => view_grid_btn.set_active(true),
        }
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        tab_row.append(&all_tab_btn);
        tab_row.append(&fav_tab_btn);
        tab_row.append(&spacer);
        tab_row.append(&view_list_btn);
        tab_row.append(&view_grid_btn);

        let filter_box = GtkBox::new(Orientation::Horizontal, 8);
        let filter_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Never)
            .hexpand(true)
            .child(&filter_box)
            .build();
        filter_scroll.set_margin_start(8);
        filter_scroll.set_margin_end(8);

        let list = ListBox::new();
        list.set_selection_mode(SelectionMode::None);
        list.add_css_class("navigation-sidebar");
        let list_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .min_content_height(200)
            .child(&list)
            .build();

        let grid = FlowBox::new();
        grid.set_selection_mode(SelectionMode::None);
        grid.set_homogeneous(true);
        grid.set_max_children_per_line(6);
        grid.set_min_children_per_line(2);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_margin_top(6);
        grid.set_margin_bottom(6);
        let grid_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .min_content_height(200)
            .child(&grid)
            .build();

        let channels_stack = Stack::new();
        channels_stack.set_transition_type(StackTransitionType::Crossfade);
        channels_stack.add_named(&list_scroll, Some("list"));
        channels_stack.add_named(&grid_scroll, Some("grid"));
        if settings.borrow().channel_view_mode() == ChannelViewMode::Grid {
            channels_stack.set_visible_child_name("grid");
        } else {
            channels_stack.set_visible_child_name("list");
        }

        let main_page = GtkBox::new(Orientation::Vertical, 0);
        main_page.set_hexpand(true);
        main_page.set_vexpand(true);
        main_page.append(&hero.root);
        main_page.append(visualizer.widget());
        main_page.append(&search);
        main_page.append(&tab_row);
        main_page.append(&filter_scroll);
        main_page.append(&channels_stack);

        let (np_page, np_cover, np_channel_label, np_track_label, np_artist_label, np_play_btn, np_visualizer) =
            build_now_playing_page(&settings);

        let content_stack = Stack::new();
        content_stack.set_transition_type(StackTransitionType::SlideLeftRight);
        content_stack.set_hexpand(true);
        content_stack.set_vexpand(true);
        content_stack.add_named(&main_page, Some("main"));
        content_stack.add_named(&np_page, Some("now"));
        content_stack.set_visible_child_name("main");

        overlay.set_child(Some(&backdrop_stack));
        overlay.add_overlay(&content_stack);
        toolbar.set_content(Some(&overlay));
        toast.set_child(Some(&toolbar));
        window.set_content(Some(&toast));

        let shared = Shared {
            window: window.clone(),
            client,
            player: player.clone(),
            state: state.clone(),
            settings: settings.clone(),
            tray_handle: tray_handle.clone(),
            toast: toast.clone(),
            list: list.clone(),
            grid: grid.clone(),
            channels_stack: channels_stack.clone(),
            filter_box: filter_box.clone(),
            search: search.clone(),
            channel_label: hero.channel_label.clone(),
            track_label: hero.track_label.clone(),
            artist_label: hero.artist_label.clone(),
            np_channel_label,
            np_track_label,
            np_artist_label,
            cover: hero.cover.clone(),
            np_cover,
            backdrop: backdrop.clone(),
            play_btn: hero.play_btn.clone(),
            np_play_btn,
            mute_btn: hero.mute_btn.clone(),
            status_dot: hero.status_dot.clone(),
            account_label: hero.account_label.clone(),
            quality_label: hero.quality_label.clone(),
            visualizer: visualizer.clone(),
            np_visualizer,
            content_stack: content_stack.clone(),
            header_title: header_title.clone(),
            login_btn: login_btn.clone(),
            settings_btn: settings_btn.clone(),
            refresh_btn: refresh_btn.clone(),
            fullscreen_btn: fullscreen_btn.clone(),
            shortcuts_btn: shortcuts_btn.clone(),
            view_list_btn: view_list_btn.clone(),
            view_grid_btn: view_grid_btn.clone(),
            fav_tab_btn: fav_tab_btn.clone(),
            all_tab_btn: all_tab_btn.clone(),
            brand_label: hero.brand_label.clone(),
            accent_label: hero.accent_label.clone(),
        };

        apply_ui_strings(&shared);
        refresh_account_label(&shared);
        refresh_quality_label(&shared);
        refresh_mute_btn(&shared);

        window.connect_close_request(clone!(
            #[strong]
            settings,
            #[strong]
            player,
            #[strong]
            tray_handle,
            move |win| {
                save_window_size(win, &settings);
                if settings.borrow().close_to_tray {
                    win.set_visible(false);
                    glib::Propagation::Stop
                } else {
                    player.shutdown();
                    shutdown_tray(&tray_handle);
                    glib::Propagation::Proceed
                }
            }
        ));

        hero.volume.set_value(settings.borrow().volume);
        hero.volume.connect_value_changed(clone!(
            #[strong]
            player,
            #[strong]
            settings,
            #[strong]
            shared,
            move |scale| {
                let v = scale.value();
                if !settings.borrow().muted {
                    let _ = player.set_volume(v);
                }
                settings.borrow_mut().volume = v;
                let _ = settings.borrow().save();
                refresh_mute_btn(&shared);
            }
        ));

        hero.play_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| toggle_playback(&shared)
        ));
        shared.np_play_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| toggle_playback(&shared)
        ));

        hero.stop_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| stop_playback(&shared)
        ));

        hero.mute_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| toggle_mute(&shared)
        ));

        login_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| show_login_dialog(&shared)
        ));
        settings_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| show_settings_dialog(&shared)
        ));
        refresh_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| load_catalog(&shared)
        ));
        fullscreen_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| toggle_now_playing(&shared)
        ));
        shortcuts_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| show_shortcuts_dialog(&shared)
        ));

        search.connect_search_changed(clone!(
            #[strong]
            shared,
            move |_| rebuild_channel_list(&shared)
        ));

        view_list_btn.connect_toggled(clone!(
            #[strong]
            shared,
            move |btn| {
                if btn.is_active() {
                    shared.settings.borrow_mut().set_channel_view_mode(ChannelViewMode::List);
                    let _ = shared.settings.borrow().save();
                    shared.channels_stack.set_visible_child_name("list");
                }
            }
        ));
        view_grid_btn.connect_toggled(clone!(
            #[strong]
            shared,
            move |btn| {
                if btn.is_active() {
                    shared.settings.borrow_mut().set_channel_view_mode(ChannelViewMode::Grid);
                    let _ = shared.settings.borrow().save();
                    shared.channels_stack.set_visible_child_name("grid");
                }
            }
        ));

        all_tab_btn.connect_toggled(clone!(
            #[strong]
            shared,
            move |btn| {
                if btn.is_active() {
                    shared.state.borrow_mut().show_favorites_only = false;
                    rebuild_channel_list(&shared);
                }
            }
        ));
        fav_tab_btn.connect_toggled(clone!(
            #[strong]
            shared,
            move |btn| {
                if btn.is_active() {
                    shared.state.borrow_mut().show_favorites_only = true;
                    rebuild_channel_list(&shared);
                }
            }
        ));

        // Keyboard shortcuts
        let controller = gtk4::EventControllerKey::new();
        controller.connect_key_pressed(clone!(
            #[strong]
            shared,
            move |_, key, _code, _mods| {
                use gdk4::Key;
                match key {
                    Key::F11 => {
                        toggle_now_playing(&shared);
                        glib::Propagation::Stop
                    }
                    Key::space => {
                        toggle_playback(&shared);
                        glib::Propagation::Stop
                    }
                    Key::s | Key::S => {
                        stop_playback(&shared);
                        glib::Propagation::Stop
                    }
                    Key::slash => {
                        shared.search.grab_focus();
                        glib::Propagation::Stop
                    }
                    Key::F5 => {
                        load_catalog(&shared);
                        glib::Propagation::Stop
                    }
                    Key::Escape => {
                        if shared.state.borrow().now_playing_mode {
                            toggle_now_playing(&shared);
                            glib::Propagation::Stop
                        } else {
                            glib::Propagation::Proceed
                        }
                    }
                    _ => glib::Propagation::Proceed,
                }
            }
        ));
        window.add_controller(controller);

        load_catalog(&shared);

        timeout_add_seconds_local(
            12,
            clone!(
                #[strong]
                shared,
                move || {
                    poll_now_playing(&shared);
                    ControlFlow::Continue
                }
            ),
        );

        timeout_add_local(
            std::time::Duration::from_millis(200),
            clone!(
                #[strong]
                shared,
                #[strong]
                tray_rx,
                #[strong]
                app,
                move || {
                    pump_tray_commands(&shared, &tray_rx, &app);
                    ControlFlow::Continue
                }
            ),
        );

        if shared.state.borrow().credentials.is_none() {
            glib::idle_add_local_once(clone!(
                #[strong]
                shared,
                move || show_login_dialog(&shared)
            ));
        }

        Self { window }
    }
}

struct Hero {
    root: GtkBox,
    channel_label: Label,
    track_label: Label,
    artist_label: Label,
    cover: Image,
    play_btn: Button,
    stop_btn: Button,
    mute_btn: Button,
    volume: Scale,
    status_dot: GtkBox,
    account_label: Label,
    quality_label: Label,
    brand_label: Label,
    accent_label: Label,
}

fn build_hero() -> Hero {
    let root = GtkBox::new(Orientation::Horizontal, 16);
    root.add_css_class("di-hero");
    root.set_margin_top(12);
    root.set_margin_bottom(4);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let cover = Image::from_icon_name("audio-x-generic");
    cover.set_pixel_size(96);
    cover.add_css_class("di-cover");

    let text_col = GtkBox::new(Orientation::Vertical, 4);
    text_col.set_hexpand(true);
    text_col.set_valign(Align::Center);

    let brand_label = Label::new(Some(&i18n::t("brand")));
    brand_label.add_css_class("di-brand");
    brand_label.set_halign(Align::Start);

    let accent_label = Label::new(Some(&i18n::t("premium-radio")));
    accent_label.add_css_class("di-accent-label");
    accent_label.set_halign(Align::Start);

    let channel_label = Label::new(Some(&i18n::t("no-channel")));
    channel_label.add_css_class("di-now-title");
    channel_label.set_halign(Align::Start);
    channel_label.set_wrap(true);

    let track_label = Label::new(Some(&i18n::t("welcome")));
    track_label.add_css_class("di-channel-name");
    track_label.add_css_class("di-fade");
    track_label.add_css_class("show");
    track_label.set_halign(Align::Start);
    track_label.set_wrap(true);

    let artist_label = Label::new(Some(&i18n::t("sign-in-hint")));
    artist_label.add_css_class("di-now-artist");
    artist_label.add_css_class("di-fade");
    artist_label.add_css_class("show");
    artist_label.set_halign(Align::Start);
    artist_label.set_wrap(true);

    let account_row = GtkBox::new(Orientation::Horizontal, 8);
    let status_dot = GtkBox::new(Orientation::Horizontal, 0);
    status_dot.set_size_request(8, 8);
    status_dot.add_css_class("di-status-dot");
    status_dot.add_css_class("offline");
    status_dot.set_valign(Align::Center);
    let account_label = Label::new(Some(&i18n::t("not-signed-in")));
    account_label.add_css_class("di-chip");
    account_label.set_halign(Align::Start);
    let quality_label = Label::new(Some("320k"));
    quality_label.add_css_class("di-chip");
    quality_label.set_halign(Align::Start);
    account_row.append(&status_dot);
    account_row.append(&account_label);
    account_row.append(&quality_label);

    text_col.append(&brand_label);
    text_col.append(&accent_label);
    text_col.append(&channel_label);
    text_col.append(&track_label);
    text_col.append(&artist_label);
    text_col.append(&account_row);

    let controls = GtkBox::new(Orientation::Vertical, 10);
    controls.set_valign(Align::Center);
    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    buttons.set_halign(Align::End);
    let play_btn = Button::from_icon_name("media-playback-start-symbolic");
    play_btn.add_css_class("di-play-btn");
    play_btn.add_css_class("circular");
    let stop_btn = Button::from_icon_name("media-playback-stop-symbolic");
    stop_btn.add_css_class("circular");
    let mute_btn = Button::from_icon_name("audio-volume-high-symbolic");
    mute_btn.add_css_class("circular");
    mute_btn.add_css_class("flat");
    buttons.append(&play_btn);
    buttons.append(&stop_btn);
    buttons.append(&mute_btn);

    let volume = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
    volume.set_draw_value(false);
    volume.set_width_request(140);

    controls.append(&buttons);
    controls.append(&volume);
    root.append(&cover);
    root.append(&text_col);
    root.append(&controls);

    Hero {
        root,
        channel_label,
        track_label,
        artist_label,
        cover,
        play_btn,
        stop_btn,
        mute_btn,
        volume,
        status_dot,
        account_label,
        quality_label,
        brand_label,
        accent_label,
    }
}

fn build_now_playing_page(
    settings: &Rc<RefCell<AppSettings>>,
) -> (GtkBox, Image, Label, Label, Label, Button, Visualizer) {
    let page = GtkBox::new(Orientation::Vertical, 16);
    page.add_css_class("di-now-playing-page");
    page.set_halign(Align::Center);
    page.set_valign(Align::Center);
    page.set_hexpand(true);
    page.set_vexpand(true);

    let cover = Image::from_icon_name("audio-x-generic");
    cover.set_pixel_size(220);
    cover.add_css_class("di-cover");
    cover.add_css_class("di-cover-lg");

    let channel = Label::new(Some(&i18n::t("no-channel")));
    channel.add_css_class("di-now-title");
    channel.set_halign(Align::Center);

    let track = Label::new(Some(&i18n::t("welcome")));
    track.add_css_class("di-channel-name");
    track.set_halign(Align::Center);

    let artist = Label::new(Some(&i18n::t("sign-in-hint")));
    artist.add_css_class("di-now-artist");
    artist.set_halign(Align::Center);
    artist.set_wrap(true);

    let viz = Visualizer::new();
    viz.set_style(settings.borrow().viz_style());
    viz.set_enabled(settings.borrow().viz_enabled);
    viz.widget().set_width_request(420);

    let play = Button::from_icon_name("media-playback-start-symbolic");
    play.add_css_class("di-play-btn");
    play.add_css_class("circular");
    play.set_halign(Align::Center);

    page.append(&cover);
    page.append(&channel);
    page.append(&track);
    page.append(&artist);
    page.append(viz.widget());
    page.append(&play);

    (page, cover, channel, track, artist, play, viz)
}

fn apply_theme(window: &adw::ApplicationWindow, theme: UiTheme) {
    for t in UiTheme::ALL {
        window.remove_css_class(t.css_class());
    }
    window.add_css_class(theme.css_class());
}

fn apply_accent(shared: &Shared, accent: AccentColor) {
    let provider = gtk4::CssProvider::new();
    let css = format!(
        "window.di-player {{
            --di-accent: {};
            --di-accent-dim: rgba({},{},{},0.18);
            --di-accent-glow: rgba({},{},{},0.45);
        }}",
        accent.css_rgb(),
        (accent.r * 255.0) as u8,
        (accent.g * 255.0) as u8,
        (accent.b * 255.0) as u8,
        (accent.r * 255.0) as u8,
        (accent.g * 255.0) as u8,
        (accent.b * 255.0) as u8,
    );
    provider.load_from_string(&css);
    let display = gtk4::prelude::WidgetExt::display(&shared.window);
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
    shared.visualizer.set_colors(accent.r, accent.g, accent.b);
    shared.np_visualizer.set_colors(accent.r, accent.g, accent.b);
}

fn apply_ui_strings(shared: &Shared) {
    shared.window.set_title(Some(&i18n::t("app-title")));
    shared
        .header_title
        .set_title(&i18n::t("app-title"));
    shared
        .header_title
        .set_subtitle(&i18n::t("app-subtitle"));
    shared.brand_label.set_text(&i18n::t("brand"));
    shared.accent_label.set_text(&i18n::t("premium-radio"));
    shared
        .search
        .set_placeholder_text(Some(&i18n::t("search-placeholder")));
    shared
        .login_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-account")));
    shared
        .settings_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-settings")));
    shared
        .refresh_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-refresh")));
    shared
        .fullscreen_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-fullscreen")));
    shared
        .shortcuts_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-shortcuts")));
    shared
        .view_list_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-view-list")));
    shared
        .view_grid_btn
        .set_tooltip_text(Some(&i18n::t("tooltip-view-grid")));
    shared.all_tab_btn.set_label(&i18n::t("tab-channels"));
    shared.fav_tab_btn.set_label(&i18n::t("tab-favorites"));
    refresh_mute_btn(shared);
    refresh_account_label(shared);
    refresh_quality_label(shared);
}

fn toggle_now_playing(shared: &Shared) {
    let mut st = shared.state.borrow_mut();
    st.now_playing_mode = !st.now_playing_mode;
    let np = st.now_playing_mode;
    drop(st);
    if np {
        shared.content_stack.set_visible_child_name("now");
        shared
            .fullscreen_btn
            .set_icon_name("view-restore-symbolic");
    } else {
        shared.content_stack.set_visible_child_name("main");
        shared
            .fullscreen_btn
            .set_icon_name("view-fullscreen-symbolic");
    }
}

fn toggle_playback(shared: &Shared) {
    match shared.player.toggle_pause() {
        Ok(PlaybackState::Playing) => {
            set_playing_ui(shared, true);
            shared.toast.add_toast(adw::Toast::new(&i18n::t("resumed")));
        }
        Ok(PlaybackState::Paused) => {
            set_playing_ui(shared, false);
            shared.toast.add_toast(adw::Toast::new(&i18n::t("paused")));
        }
        Ok(_) => shared
            .toast
            .add_toast(adw::Toast::new(&i18n::t("pick-channel"))),
        Err(err) => shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
            "playback-error",
            "error",
            &err.to_string(),
        ))),
    }
}

fn stop_playback(shared: &Shared) {
    let _ = shared.player.stop();
    set_meta_labels(shared, &i18n::t("stopped"), &i18n::t("choose-channel"));
    shared.state.borrow_mut().track_label.clear();
    set_playing_ui(shared, false);
}

fn toggle_mute(shared: &Shared) {
    let muted = {
        let mut s = shared.settings.borrow_mut();
        if s.muted {
            s.muted = false;
            let _ = shared.player.set_volume(s.volume);
            false
        } else {
            s.volume_before_mute = s.volume;
            s.muted = true;
            let _ = shared.player.set_volume(0.0);
            true
        }
    };
    let _ = shared.settings.borrow().save();
    let _ = muted;
    refresh_mute_btn(shared);
}

fn refresh_mute_btn(shared: &Shared) {
    if shared.settings.borrow().muted {
        shared.mute_btn.set_icon_name("audio-volume-muted-symbolic");
        shared
            .mute_btn
            .set_tooltip_text(Some(&i18n::t("tooltip-unmute")));
    } else {
        shared.mute_btn.set_icon_name("audio-volume-high-symbolic");
        shared
            .mute_btn
            .set_tooltip_text(Some(&i18n::t("tooltip-mute")));
    }
}

fn set_meta_labels(shared: &Shared, track: &str, artist: &str) {
    crossfade_label(&shared.track_label, track);
    crossfade_label(&shared.artist_label, artist);
    shared.np_track_label.set_text(track);
    shared.np_artist_label.set_text(artist);
}

fn crossfade_label(label: &Label, text: &str) {
    label.remove_css_class("show");
    let label = label.clone();
    let text = text.to_string();
    glib::timeout_add_local_once(std::time::Duration::from_millis(90), move || {
        label.set_text(&text);
        label.add_css_class("show");
    });
}

fn load_catalog(shared: &Shared) {
    let client = shared.client.clone();
    let quality_key = shared.settings.borrow().stream_quality().api_key().to_string();
    let (tx, rx) = async_channel::bounded::<Result<Catalog, String>>(1);
    thread::spawn(move || {
        let result = (|| {
            Ok(Catalog {
                channels: client.channels().map_err(|e| e.to_string())?,
                filters: client.channel_filters().map_err(|e| e.to_string())?,
                playlists: client
                    .stream_channels(&quality_key)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|s| (s.key, s.playlist))
                    .collect(),
            })
        })();
        let _ = tx.send_blocking(result);
    });

    let shared = shared.clone();
    glib::spawn_future_local(async move {
        let Ok(result) = rx.recv().await else {
            return;
        };
        match result {
            Ok(catalog) => {
                {
                    let mut st = shared.state.borrow_mut();
                    st.active_filter = catalog.filters.first().map(|f| f.id);
                    st.channels = catalog.channels;
                    st.filters = catalog.filters;
                    st.playlists = catalog.playlists;
                }
                rebuild_filters(&shared);
                rebuild_channel_list(&shared);
                let n = shared.state.borrow().channels.len() as i64;
                shared
                    .toast
                    .add_toast(adw::Toast::new(&i18n::t_n("loaded-channels", "n", n)));
            }
            Err(err) => {
                shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
                    "failed-channels",
                    "error",
                    &err,
                )));
            }
        }
    });
}

struct Catalog {
    channels: Vec<Channel>,
    filters: Vec<ChannelFilter>,
    playlists: HashMap<String, String>,
}

fn rebuild_filters(shared: &Shared) {
    while let Some(child) = shared.filter_box.first_child() {
        shared.filter_box.remove(&child);
    }

    let mut filters = shared.state.borrow().filters.clone();
    filters.sort_by_key(|f| match f.name.as_str() {
        "Popular" => 0,
        "All" => 1,
        "New" => 2,
        _ => 10,
    });

    let active = shared.state.borrow().active_filter;
    for filter in filters.into_iter().take(14) {
        let btn = Button::with_label(&filter.name);
        btn.add_css_class("pill");
        if Some(filter.id) == active {
            btn.add_css_class("suggested-action");
        }
        let filter_id = filter.id;
        btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| {
                shared.state.borrow_mut().active_filter = Some(filter_id);
                rebuild_filters(&shared);
                rebuild_channel_list(&shared);
            }
        ));
        shared.filter_box.append(&btn);
    }
}

fn filtered_channels(shared: &Shared) -> Vec<Channel> {
    let st = shared.state.borrow();
    let query = shared.search.text().trim().to_lowercase();
    let favorites_only = st.show_favorites_only;
    let favorites: HashSet<String> = shared
        .settings
        .borrow()
        .favorites
        .iter()
        .cloned()
        .collect();
    let allowed: Option<HashSet<u64>> = if favorites_only {
        None
    } else {
        st.active_filter.and_then(|fid| {
            st.filters
                .iter()
                .find(|f| f.id == fid)
                .map(|f| f.channels.iter().map(|c| c.id).collect())
        })
    };

    let mut channels: Vec<Channel> = st
        .channels
        .iter()
        .filter(|ch| {
            if favorites_only && !favorites.contains(&ch.key) {
                return false;
            }
            if let Some(set) = &allowed {
                if !set.contains(&ch.id) {
                    return false;
                }
            }
            if query.is_empty() {
                true
            } else {
                ch.name.to_lowercase().contains(&query)
                    || ch.key.to_lowercase().contains(&query)
                    || ch.short_description().to_lowercase().contains(&query)
            }
        })
        .cloned()
        .collect();
    channels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    channels
}

fn rebuild_channel_list(shared: &Shared) {
    while let Some(child) = shared.list.first_child() {
        shared.list.remove(&child);
    }
    while let Some(child) = shared.grid.first_child() {
        shared.grid.remove(&child);
    }

    let channels = filtered_channels(shared);
    if shared.state.borrow().show_favorites_only && channels.is_empty() {
        let empty = Label::new(Some(&i18n::t("no-favorites")));
        empty.add_css_class("di-now-artist");
        empty.set_margin_top(24);
        empty.set_halign(Align::Center);
        let row = ListBoxRow::new();
        row.set_child(Some(&empty));
        row.set_activatable(false);
        shared.list.append(&row);
        return;
    }

    for channel in channels {
        shared.list.append(&build_channel_row(&channel, shared));
        shared.grid.insert(&build_channel_tile(&channel, shared), -1);
    }
}

fn build_channel_row(channel: &Channel, shared: &Shared) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_activatable(true);
    row.add_css_class("di-channel-row");

    let box_ = GtkBox::new(Orientation::Horizontal, 12);
    let image = Image::from_icon_name("audio-x-generic");
    image.set_pixel_size(48);
    image.add_css_class("di-cover-sm");
    if let Some(url) = channel.image_url(96) {
        load_image_async(&image, None, shared.client.clone(), url, false);
    }

    let texts = GtkBox::new(Orientation::Vertical, 2);
    texts.set_hexpand(true);
    let name = Label::new(Some(&channel.name));
    name.add_css_class("di-channel-name");
    name.set_halign(Align::Start);
    let desc = Label::new(Some(channel.short_description()));
    desc.add_css_class("di-channel-desc");
    desc.set_halign(Align::Start);
    desc.set_ellipsize(pango::EllipsizeMode::End);
    texts.append(&name);
    texts.append(&desc);

    let fav = favorite_button(channel, shared);
    let play = Button::from_icon_name("media-playback-start-symbolic");
    play.add_css_class("circular");
    play.add_css_class("flat");

    let channel_c = channel.clone();
    play.connect_clicked(clone!(
        #[strong]
        shared,
        #[strong]
        channel_c,
        move |_| play_channel(&channel_c, &shared)
    ));
    row.connect_activate(clone!(
        #[strong]
        shared,
        #[strong]
        channel_c,
        move |_| play_channel(&channel_c, &shared)
    ));

    box_.append(&image);
    box_.append(&texts);
    box_.append(&fav);
    box_.append(&play);
    row.set_child(Some(&box_));
    row
}

fn build_channel_tile(channel: &Channel, shared: &Shared) -> FlowBoxChild {
    let child = FlowBoxChild::new();
    let box_ = GtkBox::new(Orientation::Vertical, 8);
    box_.add_css_class("di-grid-tile");
    box_.set_size_request(140, 180);

    let image = Image::from_icon_name("audio-x-generic");
    image.set_pixel_size(96);
    image.add_css_class("di-cover-sm");
    image.set_halign(Align::Center);
    if let Some(url) = channel.image_url(192) {
        load_image_async(&image, None, shared.client.clone(), url, false);
    }

    let name = Label::new(Some(&channel.name));
    name.add_css_class("di-channel-name");
    name.set_halign(Align::Center);
    name.set_ellipsize(pango::EllipsizeMode::End);
    name.set_max_width_chars(16);

    let row = GtkBox::new(Orientation::Horizontal, 4);
    row.set_halign(Align::Center);
    let fav = favorite_button(channel, shared);
    let play = Button::from_icon_name("media-playback-start-symbolic");
    play.add_css_class("circular");
    play.add_css_class("flat");
    let channel_c = channel.clone();
    play.connect_clicked(clone!(
        #[strong]
        shared,
        #[strong]
        channel_c,
        move |_| play_channel(&channel_c, &shared)
    ));
    row.append(&fav);
    row.append(&play);

    box_.append(&image);
    box_.append(&name);
    box_.append(&row);

    let gesture = gtk4::GestureClick::new();
    let channel_c = channel.clone();
    gesture.connect_released(clone!(
        #[strong]
        shared,
        #[strong]
        channel_c,
        move |_, _, _, _| play_channel(&channel_c, &shared)
    ));
    box_.add_controller(gesture);

    child.set_child(Some(&box_));
    child
}

fn favorite_button(channel: &Channel, shared: &Shared) -> Button {
    let is_fav = shared.settings.borrow().is_favorite(&channel.key);
    let btn = Button::from_icon_name(if is_fav {
        "starred-symbolic"
    } else {
        "non-starred-symbolic"
    });
    btn.add_css_class("flat");
    btn.add_css_class("circular");
    btn.set_tooltip_text(Some(&i18n::t(if is_fav {
        "favorite-remove"
    } else {
        "favorite-add"
    })));
    let key = channel.key.clone();
    btn.connect_clicked(clone!(
        #[strong]
        shared,
        move |btn| {
            let now_fav = shared.settings.borrow_mut().toggle_favorite(&key);
            let _ = shared.settings.borrow().save();
            btn.set_icon_name(if now_fav {
                "starred-symbolic"
            } else {
                "non-starred-symbolic"
            });
            btn.set_tooltip_text(Some(&i18n::t(if now_fav {
                "favorite-remove"
            } else {
                "favorite-add"
            })));
            if shared.state.borrow().show_favorites_only {
                rebuild_channel_list(&shared);
            }
        }
    ));
    btn
}

fn play_channel(channel: &Channel, shared: &Shared) {
    let creds = shared.state.borrow().credentials.clone();
    let Some(creds) = creds else {
        shared
            .toast
            .add_toast(adw::Toast::new(&i18n::t("sign-in-first")));
        show_login_dialog(shared);
        return;
    };

    shared.channel_label.set_text(&channel.name);
    shared.np_channel_label.set_text(&channel.name);
    set_meta_labels(
        shared,
        &i18n::t("buffering"),
        channel.short_description(),
    );
    if let Some(url) = channel.image_url(512) {
        load_cover_with_effects(shared, url);
    }

    {
        let mut st = shared.state.borrow_mut();
        st.current = Some(channel.clone());
        st.track_label = format!("Playing {}", channel.name);
    }
    shared.settings.borrow_mut().last_channel_key = Some(channel.key.clone());
    let _ = shared.settings.borrow().save();
    set_playing_ui(shared, true);

    let quality = shared.settings.borrow().stream_quality();
    let playlist = DiClient::playlist_url(&channel.key, quality.api_key());

    let client = shared.client.clone();
    let player = shared.player.clone();
    let listen_key = creds.listen_key.clone();
    let channel_name = channel.name.clone();
    let quality_label = quality.short_label().to_string();
    let shared = shared.clone();

    let (tx, rx) = async_channel::bounded::<Result<String, String>>(1);
    thread::spawn(move || {
        let result = client
            .resolve_stream_url(&playlist, &listen_key)
            .map_err(|e| e.to_string());
        let _ = tx.send_blocking(result);
    });

    glib::spawn_future_local(async move {
        let Ok(result) = rx.recv().await else {
            return;
        };
        match result {
            Ok(url) => {
                if let Err(err) = player.play_url(&url) {
                    set_meta_labels(&shared, &i18n::t("playback-failed"), &err.to_string());
                    set_playing_ui(&shared, false);
                    shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
                        "mpv-error",
                        "error",
                        &err.to_string(),
                    )));
                } else {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("channel", channel_name.as_str());
                    args.set("quality", quality_label.as_str());
                    set_meta_labels(
                        &shared,
                        &i18n::t("on-air"),
                        &i18n::t_args("streaming", Some(&args)),
                    );
                    set_playing_ui(&shared, true);
                    shared
                        .toast
                        .add_toast(adw::Toast::new(&i18n::t_args("playing-toast", Some(&args))));
                }
            }
            Err(err) => {
                set_meta_labels(&shared, &i18n::t("unable-stream"), &err);
                set_playing_ui(&shared, false);
                shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
                    "stream-error",
                    "error",
                    &err,
                )));
            }
        }
    });
}

fn poll_now_playing(shared: &Shared) {
    let channel_id = match shared.state.borrow().current.as_ref().map(|c| c.id) {
        Some(id) => id,
        None => return,
    };
    let client = shared.client.clone();
    let (tx, rx) = async_channel::bounded(1);
    thread::spawn(move || {
        let _ = tx.send_blocking(client.currently_playing().ok());
    });
    let shared = shared.clone();
    glib::spawn_future_local(async move {
        let Ok(payload) = rx.recv().await else {
            return;
        };
        if let Some(list) = payload {
            if let Some(item) = list.into_iter().find(|i| i.channel_id == channel_id) {
                set_meta_labels(
                    &shared,
                    &item.track.display_title,
                    &item.track.display_artist,
                );
                shared.state.borrow_mut().track_label = format!(
                    "{} — {}",
                    item.track.display_artist, item.track.display_title
                );
                let playing = shared.player.state() == PlaybackState::Playing;
                sync_tray(&shared, playing);
            }
        }
    });
}

fn pump_tray_commands(
    shared: &Shared,
    tray_rx: &Rc<RefCell<Receiver<TrayCommand>>>,
    app: &adw::Application,
) {
    while let Ok(cmd) = tray_rx.borrow_mut().try_recv() {
        match cmd {
            TrayCommand::ToggleWindow => {
                if shared.window.is_visible() {
                    save_window_size(&shared.window, &shared.settings);
                    shared.window.set_visible(false);
                } else {
                    shared.window.present();
                }
            }
            TrayCommand::PlayPause => {
                if let Ok(st) = shared.player.toggle_pause() {
                    set_playing_ui(shared, st == PlaybackState::Playing);
                }
            }
            TrayCommand::Stop => stop_playback(shared),
            TrayCommand::Quit => {
                shared.player.shutdown();
                shutdown_tray(&shared.tray_handle);
                app.quit();
            }
        }
    }
}

fn load_image_async(
    image: &Image,
    backdrop: Option<&Picture>,
    client: Arc<DiClient>,
    url: String,
    with_effects: bool,
) {
    let image = image.clone();
    let backdrop = backdrop.cloned();
    let (tx, rx) = async_channel::bounded::<Option<(Option<AccentColor>, Texture, Option<Texture>)>>(1);
    thread::spawn(move || {
        let result = client.fetch_bytes(&url).ok().and_then(|bytes| {
            if with_effects {
                cover_fx::process_cover(&bytes).map(|(a, sharp, blur)| {
                    (Some(a), sharp, Some(blur))
                })
            } else {
                let bytes = glib::Bytes::from(&bytes[..]);
                Texture::from_bytes(&bytes)
                    .ok()
                    .map(|t| (None, t, None))
            }
        });
        let _ = tx.send_blocking(result);
    });
    glib::spawn_future_local(async move {
        if let Ok(Some((accent, sharp, blur))) = rx.recv().await {
            image.set_paintable(Some(&sharp));
            if let (Some(bd), Some(blur_tex)) = (backdrop, blur) {
                bd.set_paintable(Some(&blur_tex));
            }
            let _ = accent;
        }
    });
}

fn load_cover_with_effects(shared: &Shared, url: String) {
    let image = shared.cover.clone();
    let np_cover = shared.np_cover.clone();
    let backdrop = shared.backdrop.clone();
    let client = shared.client.clone();
    let shared_c = shared.clone();
    let (tx, rx) = async_channel::bounded::<Option<(AccentColor, Texture, Texture)>>(1);
    thread::spawn(move || {
        let result = client
            .fetch_bytes(&url)
            .ok()
            .and_then(|bytes| cover_fx::process_cover(&bytes));
        let _ = tx.send_blocking(result);
    });
    glib::spawn_future_local(async move {
        if let Ok(Some((accent, sharp, blur))) = rx.recv().await {
            image.set_paintable(Some(&sharp));
            np_cover.set_paintable(Some(&sharp));
            backdrop.set_paintable(Some(&blur));
            apply_accent(&shared_c, accent);
        }
    });
}

fn set_playing_ui(shared: &Shared, playing: bool) {
    let icon = if playing {
        "media-playback-pause-symbolic"
    } else {
        "media-playback-start-symbolic"
    };
    shared.play_btn.set_icon_name(icon);
    shared.np_play_btn.set_icon_name(icon);
    if playing {
        shared.status_dot.remove_css_class("offline");
        shared.cover.add_css_class("playing");
        shared.np_cover.add_css_class("playing");
    } else {
        shared.cover.remove_css_class("playing");
        shared.np_cover.remove_css_class("playing");
        if shared.player.state() != PlaybackState::Playing {
            shared.status_dot.add_css_class("offline");
        }
    }
    shared.visualizer.set_playing(playing);
    shared.np_visualizer.set_playing(playing);
    sync_tray(shared, playing);
}

fn sync_tray(shared: &Shared, playing: bool) {
    let st = shared.state.borrow();
    let channel = st
        .current
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let track = st.track_label.clone();
    if let Some(handle) = shared.tray_handle.lock().as_ref() {
        tray::update_now_playing(handle, playing, &channel, &track);
    }
}

fn save_window_size(win: &impl GtkWindowExt, settings: &Rc<RefCell<AppSettings>>) {
    let (w, h) = win.default_size();
    if w > 0 && h > 0 {
        let mut s = settings.borrow_mut();
        s.window_width = w;
        s.window_height = h;
        let _ = s.save();
    }
}

fn shutdown_tray(tray_handle: &Arc<Mutex<Option<TrayHandle<DiTray>>>>) {
    if let Some(handle) = tray_handle.lock().take() {
        thread::spawn(move || {
            handle.shutdown().wait();
        });
    }
}

fn refresh_account_label(shared: &Shared) {
    let creds = shared.state.borrow().credentials.clone();
    match creds {
        Some(c) => {
            let name = if c.display_name.is_empty() {
                c.email.clone()
            } else {
                c.display_name.clone()
            };
            shared
                .account_label
                .set_text(&i18n::t_str("premium-chip", "name", &name));
            shared.status_dot.remove_css_class("offline");
        }
        None => {
            shared.account_label.set_text(&i18n::t("not-signed-in"));
            shared.status_dot.add_css_class("offline");
        }
    }
}

fn refresh_quality_label(shared: &Shared) {
    let q = shared.settings.borrow().stream_quality();
    shared.quality_label.set_text(q.short_label());
    shared.quality_label.set_tooltip_text(Some(&i18n::t_str(
        "quality-tooltip",
        "quality",
        q.label(),
    )));
}

fn show_shortcuts_dialog(shared: &Shared) {
    let body = format!(
        "F11 — {}\nSpace — {}\nS — {}\n/ — {}\nF5 — {}\nEsc — {}",
        i18n::t("shortcut-fullscreen"),
        i18n::t("shortcut-play"),
        i18n::t("shortcut-stop"),
        i18n::t("shortcut-search"),
        i18n::t("shortcut-refresh"),
        i18n::t("tooltip-fullscreen"),
    );
    let dialog = adw::AlertDialog::new(Some(&i18n::t("shortcuts-title")), Some(&body));
    dialog.add_response("ok", "OK");
    dialog.set_default_response(Some("ok"));
    dialog.set_close_response("ok");
    dialog.present(Some(&shared.window));
}

fn show_settings_dialog(shared: &Shared) {
    let dialog = adw::AlertDialog::new(
        Some(&i18n::t("settings-title")),
        Some(&i18n::t("settings-body")),
    );

    let quality_model = StringList::new(&[
        StreamQuality::PremiumHigh.label(),
        StreamQuality::Premium.label(),
        StreamQuality::PremiumMedium.label(),
    ]);
    let quality_row = adw::ComboRow::new();
    quality_row.set_title(&i18n::t("bitrate"));
    quality_row.set_subtitle(&i18n::t("bitrate-sub"));
    quality_row.set_model(Some(&quality_model));
    quality_row.set_selected(shared.settings.borrow().stream_quality().index());

    let viz_labels: Vec<String> = VizStyle::ALL.iter().map(|s| i18n::t(s.label_key())).collect();
    let viz_refs: Vec<&str> = viz_labels.iter().map(|s| s.as_str()).collect();
    let viz_model = StringList::new(&viz_refs);
    let viz_row = adw::ComboRow::new();
    viz_row.set_title(&i18n::t("viz-style"));
    viz_row.set_subtitle(&i18n::t("viz-style-sub"));
    viz_row.set_model(Some(&viz_model));
    viz_row.set_selected(shared.settings.borrow().viz_style().index());

    let viz_enabled = adw::SwitchRow::new();
    viz_enabled.set_title(&i18n::t("viz-enabled"));
    viz_enabled.set_subtitle(&i18n::t("viz-enabled-sub"));
    viz_enabled.set_active(shared.settings.borrow().viz_enabled);

    let theme_labels: Vec<String> = UiTheme::ALL.iter().map(|t| i18n::t(t.label_key())).collect();
    let theme_refs: Vec<&str> = theme_labels.iter().map(|s| s.as_str()).collect();
    let theme_model = StringList::new(&theme_refs);
    let theme_row = adw::ComboRow::new();
    theme_row.set_title(&i18n::t("theme"));
    theme_row.set_subtitle(&i18n::t("theme-sub"));
    theme_row.set_model(Some(&theme_model));
    theme_row.set_selected(shared.settings.borrow().ui_theme().index());

    let view_model = StringList::new(&[&i18n::t("view-list"), &i18n::t("view-grid")]);
    let view_row = adw::ComboRow::new();
    view_row.set_title(&i18n::t("channel-view"));
    view_row.set_subtitle(&i18n::t("channel-view-sub"));
    view_row.set_model(Some(&view_model));
    view_row.set_selected(shared.settings.borrow().channel_view_mode().index());

    let locale_labels: Vec<String> = AppLocale::ALL
        .iter()
        .map(|l| i18n::t(l.label_key()))
        .collect();
    let locale_refs: Vec<&str> = locale_labels.iter().map(|s| s.as_str()).collect();
    let locale_model = StringList::new(&locale_refs);
    let locale_row = adw::ComboRow::new();
    locale_row.set_title(&i18n::t("language"));
    locale_row.set_subtitle(&i18n::t("language-sub"));
    locale_row.set_model(Some(&locale_model));
    locale_row.set_selected(shared.settings.borrow().app_locale().index());

    let tray_row = adw::SwitchRow::new();
    tray_row.set_title(&i18n::t("close-to-tray"));
    tray_row.set_subtitle(&i18n::t("close-to-tray-sub"));
    tray_row.set_active(shared.settings.borrow().close_to_tray);

    let playback = adw::PreferencesGroup::new();
    playback.set_title(&i18n::t("group-playback"));
    playback.add(&quality_row);

    let appearance = adw::PreferencesGroup::new();
    appearance.set_title(&i18n::t("group-appearance"));
    appearance.add(&theme_row);
    appearance.add(&viz_enabled);
    appearance.add(&viz_row);
    appearance.add(&view_row);

    let language = adw::PreferencesGroup::new();
    language.set_title(&i18n::t("group-language"));
    language.add(&locale_row);

    let behavior = adw::PreferencesGroup::new();
    behavior.set_title(&i18n::t("group-behavior"));
    behavior.add(&tray_row);

    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.append(&playback);
    box_.append(&appearance);
    box_.append(&language);
    box_.append(&behavior);
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(320)
        .max_content_height(480)
        .propagate_natural_height(true)
        .child(&box_)
        .build();
    dialog.set_extra_child(Some(&scroll));
    dialog.add_response("cancel", &i18n::t("cancel"));
    dialog.add_response("save", &i18n::t("save"));
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("cancel");

    dialog.connect_response(
        None,
        clone!(
            #[strong]
            shared,
            #[strong]
            quality_row,
            #[strong]
            viz_row,
            #[strong]
            viz_enabled,
            #[strong]
            theme_row,
            #[strong]
            view_row,
            #[strong]
            locale_row,
            #[strong]
            tray_row,
            move |_, response| {
                if response != "save" {
                    return;
                }
                let selected_q = StreamQuality::from_index(quality_row.selected());
                let selected_viz = VizStyle::from_index(viz_row.selected());
                let selected_theme = UiTheme::from_index(theme_row.selected());
                let selected_view = ChannelViewMode::from_index(view_row.selected());
                let selected_locale = AppLocale::from_index(locale_row.selected());
                let viz_on = viz_enabled.is_active();
                let close_to_tray = tray_row.is_active();
                let previous_q = shared.settings.borrow().stream_quality();
                let previous_locale = shared.settings.borrow().app_locale();
                {
                    let mut s = shared.settings.borrow_mut();
                    s.set_stream_quality(selected_q);
                    s.set_viz_style(selected_viz);
                    s.viz_enabled = viz_on;
                    s.set_ui_theme(selected_theme);
                    s.set_channel_view_mode(selected_view);
                    s.set_app_locale(selected_locale);
                    s.close_to_tray = close_to_tray;
                    let _ = s.save();
                }

                apply_theme(&shared.window, selected_theme);
                shared.visualizer.set_style(selected_viz);
                shared.np_visualizer.set_style(selected_viz);
                shared.visualizer.set_enabled(viz_on);
                shared.np_visualizer.set_enabled(viz_on);

                match selected_view {
                    ChannelViewMode::List => {
                        shared.view_list_btn.set_active(true);
                        shared.channels_stack.set_visible_child_name("list");
                    }
                    ChannelViewMode::Grid => {
                        shared.view_grid_btn.set_active(true);
                        shared.channels_stack.set_visible_child_name("grid");
                    }
                }

                if selected_locale != previous_locale {
                    i18n::set_locale(selected_locale);
                    apply_ui_strings(&shared);
                    if let Some(handle) = shared.tray_handle.lock().as_ref() {
                        tray::refresh_menu(handle);
                    }
                }

                refresh_quality_label(&shared);

                if selected_q != previous_q {
                    shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
                        "quality-set",
                        "quality",
                        selected_q.label(),
                    )));
                    let restart = shared.state.borrow().current.clone();
                    load_catalog(&shared);
                    if let Some(channel) = restart {
                        glib::timeout_add_local_once(
                            std::time::Duration::from_millis(400),
                            clone!(
                                #[strong]
                                shared,
                                move || play_channel(&channel, &shared)
                            ),
                        );
                    }
                } else {
                    shared
                        .toast
                        .add_toast(adw::Toast::new(&i18n::t("settings-saved")));
                }
            }
        ),
    );

    dialog.present(Some(&shared.window));
}

fn show_login_dialog(shared: &Shared) {
    let dialog = adw::AlertDialog::new(
        Some(&i18n::t("login-title")),
        Some(&i18n::t("login-body")),
    );

    let email = adw::EntryRow::new();
    email.set_title(&i18n::t("email"));
    let password = adw::PasswordEntryRow::new();
    password.set_title(&i18n::t("password"));
    let listen_key = adw::EntryRow::new();
    listen_key.set_title(&i18n::t("listen-key"));

    if let Some(c) = shared.state.borrow().credentials.as_ref() {
        email.set_text(&c.email);
        listen_key.set_text(&c.listen_key);
    }

    let group = adw::PreferencesGroup::new();
    group.add(&email);
    group.add(&password);
    group.add(&listen_key);
    dialog.set_extra_child(Some(&group));
    dialog.add_response("cancel", &i18n::t("cancel"));
    dialog.add_response("logout", &i18n::t("sign-out"));
    dialog.add_response("login", &i18n::t("sign-in"));
    dialog.set_response_appearance("login", adw::ResponseAppearance::Suggested);
    dialog.set_response_appearance("logout", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("login"));
    dialog.set_close_response("cancel");

    dialog.connect_response(
        None,
        clone!(
            #[strong]
            shared,
            #[strong]
            email,
            #[strong]
            password,
            #[strong]
            listen_key,
            move |_, response| {
                match response {
                    "logout" => {
                        let _ = config::clear_credentials();
                        shared.state.borrow_mut().credentials = None;
                        refresh_account_label(&shared);
                        shared
                            .toast
                            .add_toast(adw::Toast::new(&i18n::t("signed-out")));
                    }
                    "login" => {
                        let key = listen_key.text().trim().to_string();
                        let user = email.text().trim().to_string();
                        let pass = password.text().to_string();

                        if !key.is_empty() && pass.is_empty() {
                            let creds = Credentials {
                                email: if user.is_empty() {
                                    "listen-key".into()
                                } else {
                                    user
                                },
                                listen_key: key,
                                api_key: String::new(),
                                member_id: 0,
                                display_name: "Premium".into(),
                            };
                            if let Err(err) = config::save_credentials(&creds) {
                                shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
                                    "could-not-save",
                                    "error",
                                    &err.to_string(),
                                )));
                                return;
                            }
                            shared.state.borrow_mut().credentials = Some(creds);
                            refresh_account_label(&shared);
                            shared
                                .toast
                                .add_toast(adw::Toast::new(&i18n::t("listen-key-saved")));
                            return;
                        }

                        if user.is_empty() || pass.is_empty() {
                            shared
                                .toast
                                .add_toast(adw::Toast::new(&i18n::t("enter-creds")));
                            return;
                        }

                        let client = shared.client.clone();
                        let (tx, rx) = async_channel::bounded(1);
                        thread::spawn(move || {
                            let _ = tx.send_blocking(client.authenticate(&user, &pass));
                        });
                        let shared = shared.clone();
                        glib::spawn_future_local(async move {
                            let Ok(result) = rx.recv().await else {
                                return;
                            };
                            match result {
                                Ok(creds) => {
                                    let _ = config::save_credentials(&creds);
                                    shared.state.borrow_mut().credentials = Some(creds);
                                    refresh_account_label(&shared);
                                    shared
                                        .toast
                                        .add_toast(adw::Toast::new(&i18n::t("signed-in")));
                                }
                                Err(err) => {
                                    shared.toast.add_toast(adw::Toast::new(&i18n::t_str(
                                        "login-failed",
                                        "error",
                                        &err.to_string(),
                                    )));
                                }
                            }
                        });
                    }
                    _ => {}
                }
            }
        ),
    );

    dialog.present(Some(&shared.window));
}
