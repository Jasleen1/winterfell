use math::field::{AsBytes, BaseElement, FieldElement};

/// Function state is set to 4 field elements or 64 bytes. 2 elements are reserved for rate
/// and 2 elements are reserved for capacity.
const STATE_WIDTH: usize = 4;
const STATE_BYTES: usize = STATE_WIDTH * 16;

/// Two elements (32-bytes) are returned as digest.
const DIGEST_SIZE: usize = 2;

/// The number of rounds is set to 14 to provide 128-bit security level.
/// computed using algorithm 7 from https://eprint.iacr.org/2020/1143.pdf
const NUM_ROUNDS: usize = 14;

// HELPER FUNCTIONS
// ================================================================================================

/// Implementation of Rescue hash function with a 4 element state and 14 rounds. Accepts a 64-byte
/// input and returns a 32-byte digest.
pub fn rescue_s(values: &[u8], result: &mut [u8]) {
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
    let mut state = [BaseElement::ZERO; STATE_WIDTH];
    #[allow(clippy::cast_ref_to_mut)]
    let state_bytes: &mut [u8; STATE_BYTES] =
        unsafe { &mut *(&state as *const _ as *mut [u8; STATE_BYTES]) };
    state_bytes[..values.len()].copy_from_slice(values);

    // apply round function 14 times; the round function implementation is based on
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
    result.copy_from_slice((&state[..DIGEST_SIZE]).as_bytes());
}

// HELPER FUNCTIONS
// ================================================================================================

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn add_constants(state: &mut [BaseElement; STATE_WIDTH], offset: usize) {
    for i in 0..STATE_WIDTH {
        state[i] += ARK[offset + i];
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_sbox(state: &mut [BaseElement; STATE_WIDTH]) {
    for i in 0..STATE_WIDTH {
        state[i] = state[i].exp(ALPHA);
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_inv_sbox(state: &mut [BaseElement; STATE_WIDTH]) {
    // TODO: optimize
    for i in 0..STATE_WIDTH {
        state[i] = state[i].exp(INV_ALPHA);
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_mds(state: &mut [BaseElement; STATE_WIDTH]) {
    let mut result = [BaseElement::default(); STATE_WIDTH];
    let mut temp = [BaseElement::default(); STATE_WIDTH];
    for i in 0..STATE_WIDTH {
        for j in 0..STATE_WIDTH {
            temp[j] = MDS[i * STATE_WIDTH + j] * state[j];
        }

        for j in 0..STATE_WIDTH {
            result[i] += temp[j];
        }
    }
    state.copy_from_slice(&result);
}

// CONSTANTS
// ================================================================================================

/// S-Box and Inverse S-Box powers;
/// computed using algorithm 6 from https://eprint.iacr.org/2020/1143.pdf
const ALPHA: u128 = 3;
const INV_ALPHA: u128 = 226854911280625642308916371969163307691;

/// Rescue MDS matrix
/// Computed using algorithm 4 from https://eprint.iacr.org/2020/1143.pdf
const MDS: [BaseElement; STATE_WIDTH * STATE_WIDTH] = [
    BaseElement::new(340282366920938463463374557953744960808),
    BaseElement::new(1080),
    BaseElement::new(340282366920938463463374557953744961147),
    BaseElement::new(40),
    BaseElement::new(340282366920938463463374557953744932377),
    BaseElement::new(42471),
    BaseElement::new(340282366920938463463374557953744947017),
    BaseElement::new(1210),
    BaseElement::new(340282366920938463463374557953744079447),
    BaseElement::new(1277640),
    BaseElement::new(340282366920938463463374557953744532108),
    BaseElement::new(33880),
    BaseElement::new(340282366920938463463374557953720263017),
    BaseElement::new(35708310),
    BaseElement::new(340282366920938463463374557953733025977),
    BaseElement::new(925771),
];

/// Rescue round constants;
/// computed using algorithm 5 from https://eprint.iacr.org/2020/1143.pdf
const ARK: [BaseElement; STATE_WIDTH * NUM_ROUNDS * 2] = [
    BaseElement::new(252629594110556276281235816992330349983),
    BaseElement::new(121163867507455621442731872354015891839),
    BaseElement::new(244623479936175870778515556108748234900),
    BaseElement::new(181999122442017949289616572388308120964),
    BaseElement::new(130035663054758320517176088024859935575),
    BaseElement::new(274932696133623013607933255959111946013),
    BaseElement::new(130096286077538976127585373664362805864),
    BaseElement::new(209506446014122131232133742654202790201),
    BaseElement::new(51912929769931267810162308005565017268),
    BaseElement::new(202610584823002946089528994694473145326),
    BaseElement::new(295992101426532309592836871256175669136),
    BaseElement::new(313404555247438968545340310449654540090),
    BaseElement::new(137671644572045862038757754124537020379),
    BaseElement::new(29113322527929260506148183779738829778),
    BaseElement::new(98634637270536166954048957710629281939),
    BaseElement::new(90484051915535813802492401077197602516),
    BaseElement::new(193753019093186599897082621380539177732),
    BaseElement::new(88328997664086495053801384396180288832),
    BaseElement::new(134379598544046716907663161480793367313),
    BaseElement::new(50911186425769400405474055284903795891),
    BaseElement::new(12945394282446072785093894845750344239),
    BaseElement::new(110650301505380365788620562912149942995),
    BaseElement::new(154214463184362737046953674082326221874),
    BaseElement::new(306646039504788072647764955304698381135),
    BaseElement::new(279745705918489041552127329708931301079),
    BaseElement::new(111293612078035530300709391234153848359),
    BaseElement::new(18110020378502034462498434861690576309),
    BaseElement::new(41797883582559360517115865611622162330),
    BaseElement::new(333888808893608021579859508112201825908),
    BaseElement::new(291192643991850989562610634125476905625),
    BaseElement::new(115042354025120848770557866862388897952),
    BaseElement::new(281483497320099569269754505499721335457),
    BaseElement::new(172898111753678285350206449646444309824),
    BaseElement::new(202661860135906394577472615378659980424),
    BaseElement::new(141885268042225970011312316000526746741),
    BaseElement::new(270195331267041521741794476882482499817),
    BaseElement::new(196457080224171120865903216527675657315),
    BaseElement::new(56730777565482395039564396246195716949),
    BaseElement::new(4886253806084919544862202000090732791),
    BaseElement::new(147384194551383352824518757380733021990),
    BaseElement::new(119476237236248181092343711369608370324),
    BaseElement::new(182869361251406039022577235058473348729),
    BaseElement::new(45308522364899994411952744852450066909),
    BaseElement::new(15438528253368638146901598290564135576),
    BaseElement::new(130060283207960095436997328133261743365),
    BaseElement::new(83953475955438079154228277940680487556),
    BaseElement::new(328659226769709797512044291035930357326),
    BaseElement::new(228749522131871685132212950281473676382),
    BaseElement::new(46194972462682851176957413491161426658),
    BaseElement::new(296333983305826854863835978241833143471),
    BaseElement::new(138957733159616849361016139528307260698),
    BaseElement::new(67842086763518777676559492559456199109),
    BaseElement::new(45580040156133202522383315452912604930),
    BaseElement::new(67567837934606680937620346425373752595),
    BaseElement::new(202860989528104560171546683198384659325),
    BaseElement::new(22630500510153322451285114937258973361),
    BaseElement::new(324160761097464842200838878419866223614),
    BaseElement::new(338466547889555546143667391979278153877),
    BaseElement::new(189171173535649401433078628567098769571),
    BaseElement::new(162173266902020502126600904559755837464),
    BaseElement::new(136209703129442038834374731074825683052),
    BaseElement::new(61998071517031804812562190829480056772),
    BaseElement::new(307309080039351604461536918194634835054),
    BaseElement::new(26708622949278137915061761772299784349),
    BaseElement::new(129516553661717764361826568456881002617),
    BaseElement::new(224023580754958002183324313900177991825),
    BaseElement::new(17590440203644538688189654586240082513),
    BaseElement::new(135610063062379124269847491297867667710),
    BaseElement::new(146865534517067293442442506551295645352),
    BaseElement::new(238139104484181583196227119098779158429),
    BaseElement::new(39300761479713744892853256947725570060),
    BaseElement::new(54114440355764484955231402374312070440),
    BaseElement::new(222758070305343916663075833184045878425),
    BaseElement::new(323840793618712078836672915700599856701),
    BaseElement::new(103586087979277053032666296091805459741),
    BaseElement::new(160263698024385270625527195046420579470),
    BaseElement::new(76620453913654705501329735586535761337),
    BaseElement::new(117793948142462197480091377165008040465),
    BaseElement::new(86998218841589258723143213495722487114),
    BaseElement::new(203188618662906890442620821687773659689),
    BaseElement::new(313098786815741054633864043424353402357),
    BaseElement::new(133085673687338880872979866135939079867),
    BaseElement::new(219888424885634764555580944265544343421),
    BaseElement::new(5893221169005427793512575133564978746),
    BaseElement::new(123830602624063632344313821515642988189),
    BaseElement::new(99030942908036387138287682010525589136),
    BaseElement::new(181549003357535890945363082242256699137),
    BaseElement::new(152424978799328476472358562493335008209),
    BaseElement::new(274481943862544603168725464029979191673),
    BaseElement::new(4975004592976331754728718693838357226),
    BaseElement::new(101850445399221640701542169338886750079),
    BaseElement::new(230325699922192981509673754024218912397),
    BaseElement::new(50419227750575087142720761582056939006),
    BaseElement::new(112444234528764731925178653200320603078),
    BaseElement::new(312169855609816651638877239277948636598),
    BaseElement::new(204255114617024487729019111502542629940),
    BaseElement::new(95797476952346525817251811755749179939),
    BaseElement::new(306977388944722094681694167558392710189),
    BaseElement::new(300754874465668732709232449646112602172),
    BaseElement::new(25567836410351071106804347269705784680),
    BaseElement::new(129659188855548935155840545784705385753),
    BaseElement::new(228441586459539470069565041053012869566),
    BaseElement::new(178382533299631576605259357906020320778),
    BaseElement::new(274458637266680353971597477639962034316),
    BaseElement::new(280059913840028448065185235205261648486),
    BaseElement::new(246537412674731137211182698562269717969),
    BaseElement::new(259930078572522349821084822750913159564),
    BaseElement::new(186061633995391650657311511040160727356),
    BaseElement::new(179777566992900315528995607912777709520),
    BaseElement::new(209753365793154515863736129686836743468),
    BaseElement::new(270445008049478596978645420017585428243),
    BaseElement::new(70998387591825316724846035292940615733),
];
