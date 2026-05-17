use reviewdesk::auth::{MemoryTokenStore, TokenKind, TokenStore};

#[test]
fn memory_token_store_round_trips_without_plain_file_storage() {
    let mut store = MemoryTokenStore::default();

    store.save(TokenKind::GitHub, "gho_token").unwrap();
    store.save(TokenKind::ChatGpt, "chatgpt_token").unwrap();

    assert_eq!(
        store.load(TokenKind::GitHub).unwrap().as_deref(),
        Some("gho_token")
    );
    assert_eq!(
        store.load(TokenKind::ChatGpt).unwrap().as_deref(),
        Some("chatgpt_token")
    );

    store.delete(TokenKind::GitHub).unwrap();
    assert!(store.load(TokenKind::GitHub).unwrap().is_none());
}
