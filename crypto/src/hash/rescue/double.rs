use crate::utils::as_bytes;
use math::field::{FieldElement, StarkField};

/// Function state is set to 6 field elements or 96 bytes; 4 elements are reserved for rate
/// and 2 elements are reserved for capacity.
const STATE_WIDTH: usize = 6;
const STATE_BYTES: usize = STATE_WIDTH * 16;

/// Two elements (32-bytes) are returned as digest.
const DIGEST_SIZE: usize = 2;

/// The number of rounds is set to 8 to provide 128-bit security level.
/// computed using algorithm 7 from https://eprint.iacr.org/2020/1143.pdf
const NUM_ROUNDS: usize = 8;

// HELPER FUNCTIONS
// ================================================================================================

/// Rescue hash function for double input; This implementation accepts a 64-byte input
/// and returns a 32-byte digest.
pub fn rescue_d(values: &[u8], result: &mut [u8]) {
    debug_assert!(
        values.len() <= 64,
        "expected 64 or fewer input bytes but received {}",
        values.len()
    );
    debug_assert!(
        result.len() == 32,
        "expected result to be exactly 32 bytes but received {}",
        result.len()
    );

    // copy values into state and set the remaining state elements to 0
    let mut state = [FieldElement::ZERO; STATE_WIDTH];
    #[allow(clippy::cast_ref_to_mut)]
    let state_bytes: &mut [u8; STATE_BYTES] =
        unsafe { &mut *(&state as *const _ as *mut [u8; STATE_BYTES]) };
    state_bytes[..values.len()].copy_from_slice(values);

    // apply round function 10 times; the round function implementation is based on
    // algorithm 3 from https://eprint.iacr.org/2020/1143.pdf
    for i in 0..NUM_ROUNDS {
        // step 1
        apply_sbox(&mut state);
        apply_mds(&mut state);
        add_constants(&mut state, i * 2 * STATE_WIDTH);

        // step 2
        apply_inv_sbox(&mut state);
        apply_mds(&mut state);
        add_constants(&mut state, (i * 2 + 1) * STATE_WIDTH);
    }

    // return the result
    result.copy_from_slice(as_bytes(&state[..DIGEST_SIZE]));
}

// HELPER FUNCTIONS
// ================================================================================================

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn add_constants(state: &mut [FieldElement; STATE_WIDTH], offset: usize) {
    for i in 0..STATE_WIDTH {
        state[i] = state[i] + ARK[offset + i];
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_sbox(state: &mut [FieldElement; STATE_WIDTH]) {
    for i in 0..STATE_WIDTH {
        state[i] = FieldElement::exp(state[i], ALPHA);
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_inv_sbox(state: &mut [FieldElement; STATE_WIDTH]) {
    // TODO: optimize
    for i in 0..STATE_WIDTH {
        state[i] = FieldElement::exp(state[i], INV_ALPHA);
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_mds(state: &mut [FieldElement; STATE_WIDTH]) {
    let mut result = [FieldElement::default(); STATE_WIDTH];
    let mut temp = [FieldElement::default(); STATE_WIDTH];
    for i in 0..STATE_WIDTH {
        for j in 0..STATE_WIDTH {
            temp[j] = MDS[i * STATE_WIDTH + j] * state[j];
        }

        for j in 0..STATE_WIDTH {
            result[i] = result[i] + temp[j];
        }
    }
    state.copy_from_slice(&result);
}

// CONSTANTS
// ================================================================================================

/// S-Box and Inverse S-Box powers;
/// computed using algorithm 6 from https://eprint.iacr.org/2020/1143.pdf
const ALPHA: u128 = 5;
const INV_ALPHA: u128 = 272225893536750770770699646362995969229;

/// Rescue MDS matrix
/// Computed using algorithm 4 from https://eprint.iacr.org/2020/1143.pdf
const MDS: [FieldElement; STATE_WIDTH * STATE_WIDTH] = [
    FieldElement::new(340282366920938463463374557953730612630),
    FieldElement::new(21493836),
    FieldElement::new(340282366920938463463374557953736934518),
    FieldElement::new(914760),
    FieldElement::new(340282366920938463463374557953744928504),
    FieldElement::new(364),
    FieldElement::new(340282366920938463463374557948521959389),
    FieldElement::new(7809407397),
    FieldElement::new(340282366920938463463374557950844620457),
    FieldElement::new(324945621),
    FieldElement::new(340282366920938463463374557953733852285),
    FieldElement::new(99463),
    FieldElement::new(340282366920938463463374556526559624596),
    FieldElement::new(2132618407920),
    FieldElement::new(340282366920938463463374557163162978137),
    FieldElement::new(88084432800),
    FieldElement::new(340282366920938463463374557950784345879),
    FieldElement::new(25095280),
    FieldElement::new(340282366920938463463374197863906102577),
    FieldElement::new(537966647357139),
    FieldElement::new(340282366920938463463374358646073999137),
    FieldElement::new(22165576349400),
    FieldElement::new(340282366920938463463374557212857010097),
    FieldElement::new(6174066262),
    FieldElement::new(340282366920938463463285966851139685903),
    FieldElement::new(132344277849702072),
    FieldElement::new(340282366920938463463325536573199985698),
    FieldElement::new(5448481182864720),
    FieldElement::new(340282366920938463463374376171390478291),
    FieldElement::new(1506472167928),
    FieldElement::new(340282366920938463441758328918057706841),
    FieldElement::new(32291274613403616174),
    FieldElement::new(340282366920938463451414421516665416977),
    FieldElement::new(1329039099788841441),
    FieldElement::new(340282366920938463463330243139804660633),
    FieldElement::new(366573514642546),
];

/// Rescue round constants;
/// computed using algorithm 5 from https://eprint.iacr.org/2020/1143.pdf
const ARK: [FieldElement; STATE_WIDTH * NUM_ROUNDS * 2] = [
    FieldElement::new(232350694689151131917165570858777669544),
    FieldElement::new(297138716840883070166239111380460167036),
    FieldElement::new(262280230220923724082396709497064092149),
    FieldElement::new(172158049344191113832187131208632037738),
    FieldElement::new(49064466045797039562408393043269857959),
    FieldElement::new(310779117230843293557874990285120450495),
    FieldElement::new(256706820970445617734149759518940865107),
    FieldElement::new(79123538858040670180278455836284339197),
    FieldElement::new(78750303544367952484014721485273250812),
    FieldElement::new(288861383492149579433903883762711410179),
    FieldElement::new(59801749333456280387477464033868461625),
    FieldElement::new(21443300235508431203706748477819269958),
    FieldElement::new(58568963110264836729315799795504150465),
    FieldElement::new(330748576252425315826992430477036516321),
    FieldElement::new(186265990460580587588657915966473647991),
    FieldElement::new(33474186560709631768594728335471560699),
    FieldElement::new(158848462530608412921046130349797355353),
    FieldElement::new(103951280788776493556470655637893338265),
    FieldElement::new(143328281743837680325887693977200434046),
    FieldElement::new(84141533915622931968833899936597847300),
    FieldElement::new(8289043147167319381038668861607412243),
    FieldElement::new(182690551456641207603161012621368395791),
    FieldElement::new(189966993584382842241685332212477020587),
    FieldElement::new(32137923394454105763485467845755642950),
    FieldElement::new(37831789571282423629213813309051107559),
    FieldElement::new(128553631204082467137622394929811125529),
    FieldElement::new(267986778741944677472811189878493395927),
    FieldElement::new(16604948458564067211433039503683613987),
    FieldElement::new(336102510949899388907937615764984494068),
    FieldElement::new(269515689098362827313089599343791905108),
    FieldElement::new(299424679105391259942771484229152481303),
    FieldElement::new(204910193356347483970850685012209050540),
    FieldElement::new(297547986861132400067173315704469727918),
    FieldElement::new(90994669428470088728996184833134573519),
    FieldElement::new(194832530917116381832912394976136685925),
    FieldElement::new(3544879195102182108390682435201981399),
    FieldElement::new(339480205126523778084089852053600037139),
    FieldElement::new(7584482258985997923597941079175892345),
    FieldElement::new(293411952222390873312400094181647328549),
    FieldElement::new(199529004542042321671242096609546451065),
    FieldElement::new(67129123347758775813781826519244753478),
    FieldElement::new(262358775581253675478636059962684988488),
    FieldElement::new(214578730175648891816936630380713062555),
    FieldElement::new(298888476681892954783673663609236117055),
    FieldElement::new(28713802418311531156758766332916445632),
    FieldElement::new(1440134829402109711440873134882900954),
    FieldElement::new(136568912729847804743104940208565395935),
    FieldElement::new(282333114631262903665175684297593586626),
    FieldElement::new(179980515973143677823617972256218090691),
    FieldElement::new(262324617228293661450608983002445445851),
    FieldElement::new(101457408539557988072857167265007764003),
    FieldElement::new(135015365700146217343913438445165565670),
    FieldElement::new(160037359781136723784361845515476884821),
    FieldElement::new(182530253870899012049936279038476084254),
    FieldElement::new(135879876810809726132885131537021449499),
    FieldElement::new(232021530889024386996643355214152586646),
    FieldElement::new(145764181560102807472161589832442506602),
    FieldElement::new(30096323905520593555387863391076216460),
    FieldElement::new(26964230850883304384940372063347292502),
    FieldElement::new(248723932438838238159920468579438468564),
    FieldElement::new(294269904099379916907622037481357861347),
    FieldElement::new(68547751515194812125080398554316505804),
    FieldElement::new(206967528806115588933607920597265054243),
    FieldElement::new(218563991130423186053843420486943196637),
    FieldElement::new(271753381570791699387473121354016967661),
    FieldElement::new(280821616954361601859332610476339898658),
    FieldElement::new(10004341245328361103806488533574675264),
    FieldElement::new(102737972201824925757345477497905200949),
    FieldElement::new(181579715086871199454198713448655357907),
    FieldElement::new(334443686013848360201749831728546200670),
    FieldElement::new(43930702221243327593116820380585481596),
    FieldElement::new(16744004758332429127464852702179311517),
    FieldElement::new(310201738135125726809998762242791360596),
    FieldElement::new(155126893730515639579436939964032992002),
    FieldElement::new(61238650483248463229462616021804212788),
    FieldElement::new(6693212157784826508674787451860949238),
    FieldElement::new(197651057967963974372308220503477603713),
    FieldElement::new(174221476673212934077040088950046690415),
    FieldElement::new(287511813733819668564695051918836002922),
    FieldElement::new(304531189544765525159398110881793396421),
    FieldElement::new(276777415462914862553995344360435589651),
    FieldElement::new(241036817921529641113885285343669990717),
    FieldElement::new(320958231309550951576801366383624382828),
    FieldElement::new(242260690344880997681123448650535822378),
    FieldElement::new(201589105262974747061391276271612166799),
    FieldElement::new(21009766855942890883171267876432289297),
    FieldElement::new(303226336248222109074995022589884483065),
    FieldElement::new(105515432862530091605210357969101266504),
    FieldElement::new(235097661610089805414814372959229370626),
    FieldElement::new(210361497167001742816223425802317150493),
    FieldElement::new(218546747003262668455051521918398855294),
    FieldElement::new(280724473534270362895764829061545243190),
    FieldElement::new(179926408118748249708833901850481685351),
    FieldElement::new(168859451670725335987760025077648496937),
    FieldElement::new(127174659756870191527945451601624140498),
    FieldElement::new(290826558340641225374953827677533570165),
];
