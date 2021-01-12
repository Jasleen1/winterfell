use super::*;

/// OBJECT ID TESTS
/// ===============================================================================================

#[test]
fn plasma_object_id_new() {
    let bytes = [
        1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];
    let oid = ObjectId::new(bytes);
    assert_eq!(oid.to_bytes(), bytes);
    assert_eq!("0102030405060708090a0b0c0d0e0f1011121314", oid.to_hex());
}

#[test]
fn plasma_object_id_rand() {
    let oid1 = ObjectId::rand();
    let oid2 = ObjectId::rand();
    assert_ne!(oid1, oid2);
}

#[test]
fn plasma_object_id_clone() {
    let oid1 = ObjectId::rand();
    let oid2 = oid1.clone();
    assert_eq!(oid1, oid2);
}

/// CLIENT TESTS
/// ===============================================================================================

#[test]
fn plasma_client_get() {
    let mut pc = PlasmaClient::new();
    pc.connect("/tmp/plasma", 0);

    let oid = ObjectId::rand();
    let data = [2u8; 16];
    let meta = vec![1, 2, 3, 4];
    pc.create_and_seal(oid.clone(), &data, &meta);

    let ob = pc.get(oid, 5);
    println!("data: {:?}", ob.data());
    println!("meta: {:?}", ob.meta());
    println!("is mutable: {}", ob.is_mutable());
    assert!(false);
}

#[test]
fn plasma_client_create_and_seal() {
    let mut pc = PlasmaClient::new();
    pc.connect("/tmp/plasma", 0);

    let oid = ObjectId::rand();
    let meta = vec![1, 2, 3, 4];
    let mut ob = pc.create(oid.clone(), 16, &meta);

    println!("is mutable: {}", ob.is_mutable());

    let data = ob.data_mut();
    for i in 0..data.len() {
        data[i] = i as u8;
    }
    pc.seal(&ob);

    let ob = pc.get(oid, 5);
    println!("data: {:?}", ob.data());
    println!("meta: {:?}", ob.meta());
    assert!(false);
}
