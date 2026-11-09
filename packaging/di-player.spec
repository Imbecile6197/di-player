Name:           di-player
Version:        0.2.0
Release:        1%{?dist}
Summary:        Native GNOME player for DI.fm premium radio
License:        GPL-3.0-or-later
URL:            https://github.com/Imbecile6197/di-player
BuildArch:      x86_64
Requires:       gtk4 libadwaita mpv

%description
DI Player is a native GNOME (GTK4 + libadwaita) client for Digitally Imported
(DI.fm) premium radio streams, with channel browser, login, visualizer,
themes, Czech/German localization, and system tray.

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/{16x16,22x22,24x24,32x32,48x48,64x64,96x96,128x128,256x256,512x512}/apps

install -m 755 %{_sourcedir}/di_player %{buildroot}%{_bindir}/di_player

install -m 644 %{_sourcedir}/com.alchymista.DiPlayer.desktop \
  %{buildroot}%{_datadir}/applications/com.alchymista.DiPlayer.desktop

install -m 644 %{_sourcedir}/com.alchymista.DiPlayer.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg
install -m 644 %{_sourcedir}/com.alchymista.DiPlayer-symbolic.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/com.alchymista.DiPlayer-symbolic.svg

for s in 16 22 24 32 48 64 96 128 256 512; do
  install -m 644 %{_sourcedir}/com.alchymista.DiPlayer-${s}.png \
    %{buildroot}%{_datadir}/icons/hicolor/${s}x${s}/apps/com.alchymista.DiPlayer.png
done
install -m 644 %{_sourcedir}/com.alchymista.DiPlayer-symbolic-64.png \
  %{buildroot}%{_datadir}/icons/hicolor/64x64/apps/com.alchymista.DiPlayer-symbolic.png

%files
%{_bindir}/di_player
%{_datadir}/applications/com.alchymista.DiPlayer.desktop
%{_datadir}/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg
%{_datadir}/icons/hicolor/scalable/apps/com.alchymista.DiPlayer-symbolic.svg
%{_datadir}/icons/hicolor/*/apps/com.alchymista.DiPlayer.png
%{_datadir}/icons/hicolor/64x64/apps/com.alchymista.DiPlayer-symbolic.png

%post
/bin/touch --no-create %{_datadir}/icons/hicolor &>/dev/null || :
/usr/bin/gtk4-update-icon-cache -f %{_datadir}/icons/hicolor &>/dev/null || \
  /usr/bin/gtk-update-icon-cache -f %{_datadir}/icons/hicolor &>/dev/null || :
/usr/bin/update-desktop-database %{_datadir}/applications &>/dev/null || :

%postun
/bin/touch --no-create %{_datadir}/icons/hicolor &>/dev/null || :
/usr/bin/gtk4-update-icon-cache -f %{_datadir}/icons/hicolor &>/dev/null || \
  /usr/bin/gtk-update-icon-cache -f %{_datadir}/icons/hicolor &>/dev/null || :
/usr/bin/update-desktop-database %{_datadir}/applications &>/dev/null || :

%changelog
* Mon Nov 09 2026 Libor Macák <jsem@libormacak.eu> - 0.2.0-1
- Visualizer, themes, CS/DE localization, monochrome Di icon
- Stop writing user hicolor index.theme (broke other GTK apps)

* Sat Nov 07 2026 Libor Macák <jsem@libormacak.eu> - 0.1.0-1
- Initial package
