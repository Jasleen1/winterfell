use crypto::hash::blake3;
use math::field::BaseElement;

#[test]
fn random_generator_draw() {
    let mut generator = super::RandomElementGenerator::new([0; 32], 0, blake3);

    let result = generator.draw::<BaseElement>();
    assert_eq!(
        result,
        BaseElement::new(257367016314067561345826246336977956381)
    );

    let result = generator.draw::<BaseElement>();
    assert_eq!(
        result,
        BaseElement::new(71356866342624880993791800984977673254)
    );

    let result = generator.draw::<BaseElement>();
    assert_eq!(
        result,
        BaseElement::new(209866678167327876517963759170433911820)
    );
}
