use super::*;

#[test]
fn plasma_get() {
    let mut pc = PlasmaClient::new();
    pc.connect("/tmp/plasma", 0);

    let oid = ObjectId::rand();
    let data = [2u8; 16];
    let meta = vec![1, 2, 3, 4];
    pc.create_and_seal(&oid, &data, &meta);

    let ob = pc.get(&oid, 5);
    println!("data: {:?}", ob.data());
    println!("meta: {:?}", ob.meta());
    assert!(false);
}

#[test]
fn plasma_create_and_seal() {
    let mut pc = PlasmaClient::new();
    pc.connect("/tmp/plasma", 0);

    let oid = ObjectId::rand();
    let meta = vec![1, 2, 3, 4];
    let mut ob = pc.create(&oid, 16, &meta);

    let data = ob.data_mut();
    for i in 0..data.len() {
        data[i] = i as u8;
    }
    pc.seal(&oid);

    let ob = pc.get(&oid, 5);
    println!("data: {:?}", ob.data());
    println!("meta: {:?}", ob.meta());
    assert!(false);
}
