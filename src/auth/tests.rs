use keyring_core::{Error, mock};

use super::keychain::{self, Removed};
use super::{LookupError, Source, lookup, store};

fn fail_next_keychain_call() {
    let entry = keychain::entry().unwrap();
    let credential: &mock::Cred = entry.as_any().downcast_ref().unwrap();
    credential.set_error(Error::NoStorageAccess("the keychain is locked".into()));
}

#[test]
fn the_key_comes_from_the_variable_then_the_keychain() {
    let (key, source) = lookup(Some("sk-from-env".to_owned())).unwrap();
    assert_eq!(key.expose(), "sk-from-env");
    assert!(matches!(source, Source::Environment));
    assert!(keyring_core::get_default_store().is_none());

    keyring_core::set_default_store(mock::Store::new().unwrap());
    let missing = lookup(None).err().unwrap();
    assert!(matches!(missing, LookupError::Missing));
    assert_eq!(
        missing.to_string(),
        "no API key: set OPENROUTER_API_KEY or run jevpipe auth set-key"
    );

    store("  sk-or-v1-first \n").unwrap();
    store("sk-or-v1-second\n").unwrap();
    for variable in [None, Some(String::new())] {
        let (key, source) = lookup(variable).unwrap();
        assert_eq!(key.expose(), "sk-or-v1-second");
        assert!(matches!(source, Source::Keychain));
    }

    fail_next_keychain_call();
    let (key, _) = lookup(Some("sk-from-env".to_owned())).unwrap();
    assert_eq!(key.expose(), "sk-from-env");
    let unavailable = lookup(None).err().unwrap();
    assert!(matches!(unavailable, LookupError::Keychain(_)));
    assert!(
        unavailable.to_string().contains("the keychain is locked"),
        "{unavailable}"
    );

    assert!(matches!(keychain::remove(), Ok(Removed::Removed)));
    assert!(matches!(keychain::remove(), Ok(Removed::NothingStored)));
    assert!(matches!(lookup(None), Err(LookupError::Missing)));
}
