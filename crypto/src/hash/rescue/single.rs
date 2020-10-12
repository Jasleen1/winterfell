use crate::utils::as_bytes;
use math::field::{f128::FieldElement, StarkField};

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

/// Rescue hash function for small inputs; This implementation accepts a 32-byte input
/// and returns a 32-byte digest.
pub fn rescue_s(values: &[u8], result: &mut [u8]) {
    debug_assert!(
        values.len() <= 32,
        "expected 32 or fewer input bytes but received {}",
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
const ALPHA: u128 = 3;
const INV_ALPHA: u128 = 26854911280625642308916371969163307691;

/// Rescue MDS matrix
/// Computed using algorithm 4 from https://eprint.iacr.org/2020/1143.pdf
const MDS: [FieldElement; STATE_WIDTH * STATE_WIDTH] = [
    FieldElement::new(340282366920938463463374557953744960808u128),
    FieldElement::new(1080u128),
    FieldElement::new(340282366920938463463374557953744961147u128),
    FieldElement::new(40u128),
    FieldElement::new(340282366920938463463374557953744932377u128),
    FieldElement::new(42471u128),
    FieldElement::new(340282366920938463463374557953744947017u128),
    FieldElement::new(1210u128),
    FieldElement::new(340282366920938463463374557953744079447u128),
    FieldElement::new(1277640u128),
    FieldElement::new(340282366920938463463374557953744532108u128),
    FieldElement::new(33880u128),
    FieldElement::new(340282366920938463463374557953720263017u128),
    FieldElement::new(35708310u128),
    FieldElement::new(340282366920938463463374557953733025977u128),
    FieldElement::new(925771u128),
];

/// Rescue round constants;
/// computed using algorithm 5 from https://eprint.iacr.org/2020/1143.pdf
const ARK: [FieldElement; STATE_WIDTH * NUM_ROUNDS * 2] = [
    FieldElement::new(252629594110556276281235816992330349983u128),
    FieldElement::new(121163867507455621442731872354015891839u128),
    FieldElement::new(244623479936175870778515556108748234900u128),
    FieldElement::new(181999122442017949289616572388308120964u128),
    FieldElement::new(130035663054758320517176088024859935575u128),
    FieldElement::new(274932696133623013607933255959111946013u128),
    FieldElement::new(130096286077538976127585373664362805864u128),
    FieldElement::new(209506446014122131232133742654202790201u128),
    FieldElement::new(51912929769931267810162308005565017268u128),
    FieldElement::new(202610584823002946089528994694473145326u128),
    FieldElement::new(295992101426532309592836871256175669136u128),
    FieldElement::new(313404555247438968545340310449654540090u128),
    FieldElement::new(137671644572045862038757754124537020379u128),
    FieldElement::new(29113322527929260506148183779738829778u128),
    FieldElement::new(98634637270536166954048957710629281939u128),
    FieldElement::new(90484051915535813802492401077197602516u128),
    FieldElement::new(193753019093186599897082621380539177732u128),
    FieldElement::new(88328997664086495053801384396180288832u128),
    FieldElement::new(134379598544046716907663161480793367313u128),
    FieldElement::new(50911186425769400405474055284903795891u128),
    FieldElement::new(12945394282446072785093894845750344239u128),
    FieldElement::new(110650301505380365788620562912149942995u128),
    FieldElement::new(154214463184362737046953674082326221874u128),
    FieldElement::new(306646039504788072647764955304698381135u128),
    FieldElement::new(279745705918489041552127329708931301079u128),
    FieldElement::new(111293612078035530300709391234153848359u128),
    FieldElement::new(18110020378502034462498434861690576309u128),
    FieldElement::new(41797883582559360517115865611622162330u128),
    FieldElement::new(333888808893608021579859508112201825908u128),
    FieldElement::new(291192643991850989562610634125476905625u128),
    FieldElement::new(115042354025120848770557866862388897952u128),
    FieldElement::new(281483497320099569269754505499721335457u128),
    FieldElement::new(172898111753678285350206449646444309824u128),
    FieldElement::new(202661860135906394577472615378659980424u128),
    FieldElement::new(141885268042225970011312316000526746741u128),
    FieldElement::new(270195331267041521741794476882482499817u128),
    FieldElement::new(196457080224171120865903216527675657315u128),
    FieldElement::new(56730777565482395039564396246195716949u128),
    FieldElement::new(4886253806084919544862202000090732791u128),
    FieldElement::new(147384194551383352824518757380733021990u128),
    FieldElement::new(119476237236248181092343711369608370324u128),
    FieldElement::new(182869361251406039022577235058473348729u128),
    FieldElement::new(45308522364899994411952744852450066909u128),
    FieldElement::new(15438528253368638146901598290564135576u128),
    FieldElement::new(130060283207960095436997328133261743365u128),
    FieldElement::new(83953475955438079154228277940680487556u128),
    FieldElement::new(328659226769709797512044291035930357326u128),
    FieldElement::new(228749522131871685132212950281473676382u128),
    FieldElement::new(46194972462682851176957413491161426658u128),
    FieldElement::new(296333983305826854863835978241833143471u128),
    FieldElement::new(138957733159616849361016139528307260698u128),
    FieldElement::new(67842086763518777676559492559456199109u128),
    FieldElement::new(45580040156133202522383315452912604930u128),
    FieldElement::new(67567837934606680937620346425373752595u128),
    FieldElement::new(202860989528104560171546683198384659325u128),
    FieldElement::new(22630500510153322451285114937258973361u128),
    FieldElement::new(324160761097464842200838878419866223614u128),
    FieldElement::new(338466547889555546143667391979278153877u128),
    FieldElement::new(189171173535649401433078628567098769571u128),
    FieldElement::new(162173266902020502126600904559755837464u128),
    FieldElement::new(136209703129442038834374731074825683052u128),
    FieldElement::new(61998071517031804812562190829480056772u128),
    FieldElement::new(307309080039351604461536918194634835054u128),
    FieldElement::new(26708622949278137915061761772299784349u128),
    FieldElement::new(129516553661717764361826568456881002617u128),
    FieldElement::new(224023580754958002183324313900177991825u128),
    FieldElement::new(17590440203644538688189654586240082513u128),
    FieldElement::new(135610063062379124269847491297867667710u128),
    FieldElement::new(146865534517067293442442506551295645352u128),
    FieldElement::new(238139104484181583196227119098779158429u128),
    FieldElement::new(39300761479713744892853256947725570060u128),
    FieldElement::new(54114440355764484955231402374312070440u128),
    FieldElement::new(222758070305343916663075833184045878425u128),
    FieldElement::new(323840793618712078836672915700599856701u128),
    FieldElement::new(103586087979277053032666296091805459741u128),
    FieldElement::new(160263698024385270625527195046420579470u128),
    FieldElement::new(76620453913654705501329735586535761337u128),
    FieldElement::new(117793948142462197480091377165008040465u128),
    FieldElement::new(86998218841589258723143213495722487114u128),
    FieldElement::new(203188618662906890442620821687773659689u128),
    FieldElement::new(313098786815741054633864043424353402357u128),
    FieldElement::new(133085673687338880872979866135939079867u128),
    FieldElement::new(219888424885634764555580944265544343421u128),
    FieldElement::new(5893221169005427793512575133564978746u128),
    FieldElement::new(123830602624063632344313821515642988189u128),
    FieldElement::new(99030942908036387138287682010525589136u128),
    FieldElement::new(181549003357535890945363082242256699137u128),
    FieldElement::new(152424978799328476472358562493335008209u128),
    FieldElement::new(274481943862544603168725464029979191673u128),
    FieldElement::new(4975004592976331754728718693838357226u128),
    FieldElement::new(101850445399221640701542169338886750079u128),
    FieldElement::new(230325699922192981509673754024218912397u128),
    FieldElement::new(50419227750575087142720761582056939006u128),
    FieldElement::new(112444234528764731925178653200320603078u128),
    FieldElement::new(312169855609816651638877239277948636598u128),
    FieldElement::new(204255114617024487729019111502542629940u128),
    FieldElement::new(95797476952346525817251811755749179939u128),
    FieldElement::new(306977388944722094681694167558392710189u128),
    FieldElement::new(300754874465668732709232449646112602172u128),
    FieldElement::new(25567836410351071106804347269705784680u128),
    FieldElement::new(129659188855548935155840545784705385753u128),
    FieldElement::new(228441586459539470069565041053012869566u128),
    FieldElement::new(178382533299631576605259357906020320778u128),
    FieldElement::new(274458637266680353971597477639962034316u128),
    FieldElement::new(280059913840028448065185235205261648486u128),
    FieldElement::new(246537412674731137211182698562269717969u128),
    FieldElement::new(259930078572522349821084822750913159564u128),
    FieldElement::new(186061633995391650657311511040160727356u128),
    FieldElement::new(179777566992900315528995607912777709520u128),
    FieldElement::new(209753365793154515863736129686836743468u128),
    FieldElement::new(270445008049478596978645420017585428243u128),
    FieldElement::new(70998387591825316724846035292940615733u128),
];
