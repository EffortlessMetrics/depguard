use depguard_test_util::{
    crypto_fixture_factory,
    uselesskey::{ChainSpec, RsaFactoryExt, RsaSpec, X509FactoryExt, negative::CorruptPem},
};

#[test]
fn rsa_fixtures_are_deterministic_and_support_negative_variants() {
    let factory = crypto_fixture_factory(concat!(
        module_path!(),
        "::rsa_fixtures_are_deterministic_and_support_negative_variants"
    ));

    let keypair = factory.rsa("signing", RsaSpec::rs256());
    let private_pem = keypair.private_key_pkcs8_pem().to_owned();
    let public_pem = keypair.public_key_spki_pem().to_owned();

    assert!(private_pem.starts_with("-----BEGIN PRIVATE KEY-----"));
    assert!(public_pem.starts_with("-----BEGIN PUBLIC KEY-----"));

    let temp_key = keypair
        .write_private_key_pkcs8_pem()
        .expect("write temp private key");
    assert!(temp_key.path().exists());

    let corrupted = keypair.private_key_pkcs8_pem_corrupt(CorruptPem::BadHeader);
    assert_ne!(corrupted, private_pem);
    assert!(corrupted.contains("CORRUPTED"));

    let mismatched_public = keypair.mismatched_public_key_spki_der();
    assert_ne!(mismatched_public, keypair.public_key_spki_der());

    factory.clear_cache();
    let regenerated = factory.rsa("signing", RsaSpec::rs256());
    assert_eq!(regenerated.private_key_pkcs8_pem(), private_pem);
    assert_eq!(regenerated.public_key_spki_pem(), public_pem);
}

#[test]
fn x509_chain_fixtures_cover_tls_file_and_negative_paths() {
    let factory = crypto_fixture_factory(concat!(
        module_path!(),
        "::x509_chain_fixtures_cover_tls_file_and_negative_paths"
    ));

    let chain = factory.x509_chain("server", ChainSpec::new("localhost"));
    let full_chain_pem = chain.full_chain_pem();

    assert!(
        chain
            .leaf_cert_pem()
            .contains("-----BEGIN CERTIFICATE-----")
    );
    assert!(
        chain
            .leaf_private_key_pkcs8_pem()
            .contains("-----BEGIN PRIVATE KEY-----")
    );
    assert_eq!(
        chain
            .chain_pem()
            .matches("-----BEGIN CERTIFICATE-----")
            .count(),
        2
    );
    assert_eq!(
        full_chain_pem
            .matches("-----BEGIN CERTIFICATE-----")
            .count(),
        3
    );

    let leaf_cert = chain.write_leaf_cert_pem().expect("write temp leaf cert");
    let chain_file = chain.write_chain_pem().expect("write temp chain pem");
    assert!(leaf_cert.path().exists());
    assert!(chain_file.path().exists());

    let wrong_host = chain.hostname_mismatch("wrong.local");
    assert_ne!(wrong_host.leaf_cert_pem(), chain.leaf_cert_pem());

    let revoked = chain.revoked_leaf();
    assert!(revoked.crl_pem().is_some());

    factory.clear_cache();
    let regenerated = factory.x509_chain("server", ChainSpec::new("localhost"));
    assert_eq!(regenerated.full_chain_pem(), full_chain_pem);
}
