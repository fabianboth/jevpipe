use keyring_core::{Error, mock};

use super::keychain::{self, Removed};
use super::{LookupError, Source, lookup, store};
use crate::provider::Provider;

fn fail_next_keychain_call(provider: Provider) {
    let entry = keychain::entry(provider).unwrap();
    let credential: &mock::Cred = entry.as_any().downcast_ref().unwrap();
    credential.set_error(Error::NoStorageAccess("the keychain is locked".into()));
}

fn stored(provider: Provider) -> Option<String> {
    lookup(provider, None)
        .ok()
        .map(|(key, _)| key.expose().to_owned())
}

#[test]
fn each_provider_has_its_own_key_from_its_variable_then_the_keychain() {
    let (key, source) = lookup(Provider::TypeSafe, Some("ts-from-env".to_owned())).unwrap();
    assert_eq!(key.expose(), "ts-from-env");
    assert!(matches!(source, Source::Environment(Provider::TypeSafe)));
    assert_eq!(source.to_string(), "from TYPESAFE_API_KEY");
    assert!(keyring_core::get_default_store().is_none());

    keyring_core::set_default_store(mock::Store::new().unwrap());
    for (provider, message) in [
        (
            Provider::OpenRouter,
            "no OpenRouter API key: set OPENROUTER_API_KEY or run jevpipe auth set-key (provider openrouter; jevpipe config set provider typesafe switches)",
        ),
        (
            Provider::TypeSafe,
            "no TypeSafe API key: set TYPESAFE_API_KEY or run jevpipe auth set-key (provider typesafe; jevpipe config set provider openrouter switches)",
        ),
    ] {
        let missing = lookup(provider, None).err().unwrap();
        assert!(matches!(missing, LookupError::Missing(found) if found == provider));
        assert_eq!(missing.to_string(), message);
    }

    store(Provider::OpenRouter, "  sk-or-v1-first \n").unwrap();
    store(Provider::OpenRouter, "sk-or-v1-second\n").unwrap();
    assert_eq!(
        stored(Provider::OpenRouter).as_deref(),
        Some("sk-or-v1-second")
    );
    assert_eq!(stored(Provider::TypeSafe), None);
    store(Provider::TypeSafe, "ts-stored\n").unwrap();
    for variable in [None, Some(String::new())] {
        let (key, source) = lookup(Provider::TypeSafe, variable).unwrap();
        assert_eq!(key.expose(), "ts-stored");
        assert!(matches!(source, Source::Keychain));
    }
    assert_eq!(
        stored(Provider::OpenRouter).as_deref(),
        Some("sk-or-v1-second")
    );

    fail_next_keychain_call(Provider::OpenRouter);
    let (key, _) = lookup(Provider::OpenRouter, Some("sk-from-env".to_owned())).unwrap();
    assert_eq!(key.expose(), "sk-from-env");
    let unavailable = lookup(Provider::OpenRouter, None).err().unwrap();
    assert!(matches!(unavailable, LookupError::Keychain(..)));
    assert!(
        unavailable.to_string().contains("the keychain is locked")
            && unavailable
                .to_string()
                .ends_with("; set OPENROUTER_API_KEY instead"),
        "{unavailable}"
    );

    assert!(matches!(
        keychain::remove(Provider::TypeSafe),
        Ok(Removed::Removed)
    ));
    assert!(matches!(
        keychain::remove(Provider::TypeSafe),
        Ok(Removed::NothingStored)
    ));
    assert_eq!(
        stored(Provider::OpenRouter).as_deref(),
        Some("sk-or-v1-second")
    );
    assert!(matches!(
        keychain::remove(Provider::OpenRouter),
        Ok(Removed::Removed)
    ));
    assert!(matches!(
        lookup(Provider::OpenRouter, None),
        Err(LookupError::Missing(Provider::OpenRouter))
    ));
}
