use di_player::api::DiClient;
use di_player::config::{clear_credentials, load_credentials, save_credentials, Credentials};

#[test]
fn client_loads_channels_and_filters() {
    let client = DiClient::new().unwrap();
    let channels = client.channels().expect("channels");
    assert!(channels.len() > 50);
    assert!(channels.iter().any(|c| c.key == "trance"));

    let filters = client.channel_filters().expect("filters");
    assert!(filters.iter().any(|f| f.name == "Popular"));
    assert!(filters.iter().any(|f| f.name == "All"));

    let playing = client.currently_playing().expect("now playing");
    assert!(!playing.is_empty());
}

#[test]
fn client_rejects_bad_login() {
    let client = DiClient::new().unwrap();
    let err = client
        .authenticate("not-a-real-user@example.com", "bad-password")
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("invalid")
            || msg.to_lowercase().contains("password")
            || msg.to_lowercase().contains("auth"),
        "unexpected error: {msg}"
    );
}

#[test]
fn credentials_roundtrip() {
    let _ = clear_credentials();
    let creds = Credentials {
        email: "test@example.com".into(),
        listen_key: "abc123listen".into(),
        api_key: "key".into(),
        member_id: 42,
        display_name: "Tester".into(),
    };
    save_credentials(&creds).unwrap();
    let loaded = load_credentials().expect("loaded");
    assert_eq!(loaded.listen_key, "abc123listen");
    assert_eq!(loaded.email, "test@example.com");
    let _ = clear_credentials();
}

#[test]
fn resolve_playlist_without_key_still_parses_pls() {
    let client = DiClient::new().unwrap();
    // Even with a dummy key, the PLS endpoint usually returns hosts; stream auth happens later.
    let url = client
        .resolve_stream_url(
            "https://listen.di.fm/premium_high/trance.pls",
            "dummy-listen-key",
        )
        .expect("pls resolve");
    assert!(url.contains("di.fm"), "{url}");
}
