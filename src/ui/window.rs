use di_player::api::{Channel, ChannelFilter, DiClient};
use di_player::config::{self, AppSettings, Credentials};
use di_player::icons::APP_ICON_NAME;
use di_player::player::{MpvPlayer, PlaybackState};
use di_player::tray::{self, DiTray, TrayCommand};
use gdk4::Texture;
use glib::{clone, timeout_add_local, timeout_add_seconds_local, ControlFlow};
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Image, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, SearchEntry, Scale, SelectionMode,
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
    filter_box: GtkBox,
    search: SearchEntry,
    channel_label: Label,
    track_label: Label,
    artist_label: Label,
    cover: Image,
    play_btn: Button,
    status_dot: GtkBox,
    account_label: Label,
}

pub struct DiPlayerWindow {
    pub window: adw::ApplicationWindow,
}

impl DiPlayerWindow {
    pub fn new(app: &adw::Application) -> Self {
        let settings = Rc::new(RefCell::new(AppSettings::load()));
        let client = Arc::new(DiClient::new().expect("HTTP client"));
        let player = Arc::new(MpvPlayer::new());
        let _ = player.set_volume(settings.borrow().volume);

        let state = Rc::new(RefCell::new(RuntimeState {
            channels: Vec::new(),
            filters: Vec::new(),
            playlists: HashMap::new(),
            credentials: config::load_credentials(),
            current: None,
            track_label: String::new(),
            active_filter: None,
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
            .title("DI Player")
            .default_width(initial_w)
            .default_height(initial_h)
            .resizable(true)
            .icon_name(APP_ICON_NAME)
            .build();
        window.set_size_request(560, 420);
        window.add_css_class("di-player");

        let toast = adw::ToastOverlay::new();
        let toolbar = adw::ToolbarView::new();

        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&adw::WindowTitle::new(
            "DI Player",
            "Digitally Imported",
        )));
        let login_btn = Button::from_icon_name("avatar-default-symbolic");
        login_btn.set_tooltip_text(Some("Premium account"));
        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some("Refresh channels"));
        header.pack_end(&login_btn);
        header.pack_end(&refresh_btn);
        toolbar.add_top_bar(&header);

        let root = GtkBox::new(Orientation::Vertical, 0);
        let hero = build_hero();
        let search = SearchEntry::new();
        search.set_placeholder_text(Some("Search channels…"));
        search.set_hexpand(true);
        search.set_margin_start(12);
        search.set_margin_end(12);
        search.set_margin_top(8);

        let filter_box = GtkBox::new(Orientation::Horizontal, 8);
        filter_box.set_margin_start(8);
        filter_box.set_margin_end(8);
        filter_box.set_margin_top(4);
        filter_box.set_margin_bottom(4);
        // Keep genre chips from forcing a huge minimum window width.
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

        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .min_content_height(200)
            .child(&list)
            .build();

        root.set_hexpand(true);
        root.set_vexpand(true);
        root.append(&hero.root);
        root.append(&search);
        root.append(&filter_scroll);
        root.append(&scroll);
        toolbar.set_content(Some(&root));
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
            filter_box: filter_box.clone(),
            search: search.clone(),
            channel_label: hero.channel_label.clone(),
            track_label: hero.track_label.clone(),
            artist_label: hero.artist_label.clone(),
            cover: hero.cover.clone(),
            play_btn: hero.play_btn.clone(),
            status_dot: hero.status_dot.clone(),
            account_label: hero.account_label.clone(),
        };

        refresh_account_label(
            &shared.account_label,
            &shared.status_dot,
            &shared.state.borrow().credentials,
        );

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
            move |scale| {
                let v = scale.value();
                let _ = player.set_volume(v);
                settings.borrow_mut().volume = v;
                let _ = settings.borrow().save();
            }
        ));

        hero.play_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| {
                match shared.player.toggle_pause() {
                    Ok(PlaybackState::Playing) => {
                        set_playing_ui(&shared, true);
                        shared
                            .toast
                            .add_toast(adw::Toast::new("Resumed"));
                    }
                    Ok(PlaybackState::Paused) => {
                        set_playing_ui(&shared, false);
                        shared.toast.add_toast(adw::Toast::new("Paused"));
                    }
                    Ok(_) => shared
                        .toast
                        .add_toast(adw::Toast::new("Pick a channel first")),
                    Err(err) => shared
                        .toast
                        .add_toast(adw::Toast::new(&format!("Playback error: {err}"))),
                }
            }
        ));

        hero.stop_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| {
                let _ = shared.player.stop();
                shared.track_label.set_text("Stopped");
                shared
                    .artist_label
                    .set_text("Choose a channel to start listening");
                shared.state.borrow_mut().track_label.clear();
                set_playing_ui(&shared, false);
            }
        ));

        login_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| show_login_dialog(&shared)
        ));

        refresh_btn.connect_clicked(clone!(
            #[strong]
            shared,
            move |_| load_catalog(&shared)
        ));

        search.connect_search_changed(clone!(
            #[strong]
            shared,
            move |_| rebuild_channel_list(&shared)
        ));

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
    volume: Scale,
    status_dot: GtkBox,
    account_label: Label,
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

    let brand = Label::new(Some("DI.FM"));
    brand.add_css_class("di-brand");
    brand.set_halign(Align::Start);

    let accent = Label::new(Some("Premium Radio"));
    accent.add_css_class("di-accent-label");
    accent.set_halign(Align::Start);

    let channel_label = Label::new(Some("No channel selected"));
    channel_label.add_css_class("di-now-title");
    channel_label.set_halign(Align::Start);
    channel_label.set_wrap(true);

    let track_label = Label::new(Some("Welcome"));
    track_label.add_css_class("di-channel-name");
    track_label.set_halign(Align::Start);
    track_label.set_wrap(true);

    let artist_label =
        Label::new(Some("Sign in with your DI.fm premium account to listen"));
    artist_label.add_css_class("di-now-artist");
    artist_label.set_halign(Align::Start);
    artist_label.set_wrap(true);

    let account_row = GtkBox::new(Orientation::Horizontal, 8);
    let status_dot = GtkBox::new(Orientation::Horizontal, 0);
    status_dot.set_size_request(8, 8);
    status_dot.add_css_class("di-status-dot");
    status_dot.add_css_class("offline");
    status_dot.set_valign(Align::Center);
    let account_label = Label::new(Some("Not signed in"));
    account_label.add_css_class("di-chip");
    account_label.set_halign(Align::Start);
    account_row.append(&status_dot);
    account_row.append(&account_label);

    text_col.append(&brand);
    text_col.append(&accent);
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
    buttons.append(&play_btn);
    buttons.append(&stop_btn);

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
        volume,
        status_dot,
        account_label,
    }
}

fn load_catalog(shared: &Shared) {
    let client = shared.client.clone();
    let (tx, rx) = async_channel::bounded::<Result<Catalog, String>>(1);
    thread::spawn(move || {
        let result = (|| {
            Ok(Catalog {
                channels: client.channels().map_err(|e| e.to_string())?,
                filters: client.channel_filters().map_err(|e| e.to_string())?,
                playlists: client
                    .stream_channels()
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
                let n = shared.state.borrow().channels.len();
                shared
                    .toast
                    .add_toast(adw::Toast::new(&format!("Loaded {n} channels")));
            }
            Err(err) => {
                shared
                    .toast
                    .add_toast(adw::Toast::new(&format!("Failed to load channels: {err}")));
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

fn rebuild_channel_list(shared: &Shared) {
    while let Some(child) = shared.list.first_child() {
        shared.list.remove(&child);
    }

    let st = shared.state.borrow();
    let query = shared.search.text().trim().to_lowercase();
    let allowed: Option<HashSet<u64>> = st.active_filter.and_then(|fid| {
        st.filters
            .iter()
            .find(|f| f.id == fid)
            .map(|f| f.channels.iter().map(|c| c.id).collect())
    });

    let mut channels: Vec<Channel> = st
        .channels
        .iter()
        .filter(|ch| {
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
    drop(st);
    channels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    for channel in channels {
        shared.list.append(&build_channel_row(&channel, shared));
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
        load_image_async(&image, shared.client.clone(), url);
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

    let play = Button::from_icon_name("media-playback-start-symbolic");
    play.add_css_class("circular");
    play.add_css_class("flat");

    let channel = channel.clone();
    play.connect_clicked(clone!(
        #[strong]
        shared,
        #[strong]
        channel,
        move |_| play_channel(&channel, &shared)
    ));
    row.connect_activate(clone!(
        #[strong]
        shared,
        #[strong]
        channel,
        move |_| play_channel(&channel, &shared)
    ));

    box_.append(&image);
    box_.append(&texts);
    box_.append(&play);
    row.set_child(Some(&box_));
    row
}

fn play_channel(channel: &Channel, shared: &Shared) {
    let creds = shared.state.borrow().credentials.clone();
    let Some(creds) = creds else {
        shared
            .toast
            .add_toast(adw::Toast::new("Sign in with your DI.fm premium account first."));
        show_login_dialog(shared);
        return;
    };

    shared.channel_label.set_text(&channel.name);
    shared.track_label.set_text("Buffering…");
    shared.artist_label.set_text(channel.short_description());
    if let Some(url) = channel.image_url(256) {
        load_image_async(&shared.cover, shared.client.clone(), url);
    }

    {
        let mut st = shared.state.borrow_mut();
        st.current = Some(channel.clone());
        st.track_label = format!("Playing {}", channel.name);
    }
    shared.settings.borrow_mut().last_channel_key = Some(channel.key.clone());
    let _ = shared.settings.borrow().save();
    set_playing_ui(shared, true);

    let playlist = shared
        .state
        .borrow()
        .playlists
        .get(&channel.key)
        .cloned()
        .unwrap_or_else(|| format!("https://listen.di.fm/premium_high/{}.pls", channel.key));

    let client = shared.client.clone();
    let player = shared.player.clone();
    let listen_key = creds.listen_key.clone();
    let channel_name = channel.name.clone();
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
                    shared.track_label.set_text("Playback failed");
                    shared.artist_label.set_text(&err.to_string());
                    set_playing_ui(&shared, false);
                    shared
                        .toast
                        .add_toast(adw::Toast::new(&format!("mpv error: {err}")));
                } else {
                    shared.track_label.set_text("On air");
                    shared
                        .artist_label
                        .set_text(&format!("Streaming {channel_name}"));
                    set_playing_ui(&shared, true);
                    shared
                        .toast
                        .add_toast(adw::Toast::new(&format!("Playing {channel_name}")));
                }
            }
            Err(err) => {
                shared.track_label.set_text("Unable to start stream");
                shared.artist_label.set_text(&err);
                set_playing_ui(&shared, false);
                shared
                    .toast
                    .add_toast(adw::Toast::new(&format!("Stream error: {err}")));
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
                shared
                    .artist_label
                    .set_text(&item.track.display_artist);
                shared
                    .track_label
                    .set_text(&item.track.display_title);
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
            TrayCommand::Stop => {
                let _ = shared.player.stop();
                shared.track_label.set_text("Stopped");
                shared
                    .artist_label
                    .set_text("Choose a channel to start listening");
                shared.state.borrow_mut().track_label.clear();
                set_playing_ui(shared, false);
            }
            TrayCommand::Quit => {
                shared.player.shutdown();
                shutdown_tray(&shared.tray_handle);
                app.quit();
            }
        }
    }
}

fn load_image_async(image: &Image, client: Arc<DiClient>, url: String) {
    let image = image.clone();
    let (tx, rx) = async_channel::bounded::<Option<Texture>>(1);
    thread::spawn(move || {
        let texture = client.fetch_bytes(&url).ok().and_then(|bytes| {
            let bytes = glib::Bytes::from(&bytes[..]);
            Texture::from_bytes(&bytes).ok()
        });
        let _ = tx.send_blocking(texture);
    });
    glib::spawn_future_local(async move {
        if let Ok(Some(tex)) = rx.recv().await {
            image.set_paintable(Some(&tex));
        }
    });
}

fn set_playing_ui(shared: &Shared, playing: bool) {
    if playing {
        shared
            .play_btn
            .set_icon_name("media-playback-pause-symbolic");
        shared.status_dot.remove_css_class("offline");
    } else {
        shared
            .play_btn
            .set_icon_name("media-playback-start-symbolic");
        if shared.player.state() != PlaybackState::Playing {
            shared.status_dot.add_css_class("offline");
        }
    }
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
        // Don't block the GTK thread on a full wait — detach shutdown.
        thread::spawn(move || {
            handle.shutdown().wait();
        });
    }
}

fn refresh_account_label(label: &Label, status_dot: &GtkBox, creds: &Option<Credentials>) {
    match creds {
        Some(c) => {
            let name = if c.display_name.is_empty() {
                c.email.clone()
            } else {
                c.display_name.clone()
            };
            label.set_text(&format!("Premium · {name}"));
            status_dot.remove_css_class("offline");
        }
        None => {
            label.set_text("Not signed in");
            status_dot.add_css_class("offline");
        }
    }
}

fn show_login_dialog(shared: &Shared) {
    let dialog = adw::AlertDialog::new(
        Some("DI.fm Premium"),
        Some(
            "Sign in with your Digitally Imported account, or paste a listen key from member settings → hardware player.",
        ),
    );

    let email = adw::EntryRow::new();
    email.set_title("Email");
    let password = adw::PasswordEntryRow::new();
    password.set_title("Password");
    let listen_key = adw::EntryRow::new();
    listen_key.set_title("Or paste listen key");

    if let Some(c) = shared.state.borrow().credentials.as_ref() {
        email.set_text(&c.email);
        listen_key.set_text(&c.listen_key);
    }

    let group = adw::PreferencesGroup::new();
    group.add(&email);
    group.add(&password);
    group.add(&listen_key);
    dialog.set_extra_child(Some(&group));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("logout", "Sign out");
    dialog.add_response("login", "Sign in");
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
                        refresh_account_label(&shared.account_label, &shared.status_dot, &None);
                        shared.toast.add_toast(adw::Toast::new("Signed out"));
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
                                shared.toast.add_toast(adw::Toast::new(&format!(
                                    "Could not save credentials: {err}"
                                )));
                                return;
                            }
                            shared.state.borrow_mut().credentials = Some(creds.clone());
                            refresh_account_label(
                                &shared.account_label,
                                &shared.status_dot,
                                &Some(creds),
                            );
                            shared.toast.add_toast(adw::Toast::new("Listen key saved"));
                            return;
                        }

                        if user.is_empty() || pass.is_empty() {
                            shared.toast.add_toast(adw::Toast::new(
                                "Enter email/password or a listen key.",
                            ));
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
                                    shared.state.borrow_mut().credentials = Some(creds.clone());
                                    refresh_account_label(
                                        &shared.account_label,
                                        &shared.status_dot,
                                        &Some(creds),
                                    );
                                    shared
                                        .toast
                                        .add_toast(adw::Toast::new("Signed in to DI.fm premium"));
                                }
                                Err(err) => {
                                    shared.toast.add_toast(adw::Toast::new(&format!(
                                        "Login failed: {err}"
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
