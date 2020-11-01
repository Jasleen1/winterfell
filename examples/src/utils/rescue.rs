use super::{are_equal, EvaluationResult};
use prover::math::field::{FieldElement, StarkField};

const STATE_WIDTH: usize = 4;
const CYCLE_LENGTH: usize = 16;

// TRACE
// ================================================================================================

pub fn apply_round(state: &mut [FieldElement], step: usize) {
    // determine which round constants to use
    let ark = ARK[step % CYCLE_LENGTH];

    // apply first half of Rescue round
    apply_sbox(state);
    apply_mds(state);
    add_constants(state, &ark, 0);

    // apply second half of Rescue round
    apply_inv_sbox(state);
    apply_mds(state);
    add_constants(state, &ark, STATE_WIDTH);
}

// CONSTRAINTS
// ================================================================================================

/// when flag = 1, enforces constraints for a single round of Rescue hash functions
pub fn enforce_round(
    result: &mut [FieldElement],
    current: &[FieldElement],
    next: &[FieldElement],
    ark: &[FieldElement],
    flag: FieldElement,
) {
    // compute the state that should result from applying the first half of Rescue round
    // to the current state of the computation
    let mut step1 = [FieldElement::ZERO; STATE_WIDTH];
    step1.copy_from_slice(current);
    apply_sbox(&mut step1);
    apply_mds(&mut step1);
    for i in 0..STATE_WIDTH {
        step1[i] = step1[i] + ark[i];
    }

    // compute the state that should result from applying the inverse for the second
    // half for Rescue round to the next step of the computation
    let mut step2 = [FieldElement::ZERO; STATE_WIDTH];
    step2.copy_from_slice(next);
    for i in 0..STATE_WIDTH {
        step2[i] = step2[i] - ark[STATE_WIDTH + i];
    }
    apply_inv_mds(&mut step2);
    apply_sbox(&mut step2);

    // make sure that the results are equal
    for i in 0..STATE_WIDTH {
        result.agg_constraint(i, flag, are_equal(step2[i], step1[i]));
    }
}

// ROUND CONSTANTS
// ================================================================================================

/// Returns Rescue round constants arranged in column-major form.
pub fn get_round_constants() -> Vec<Vec<FieldElement>> {
    let mut constants = Vec::new();
    for _ in 0..(STATE_WIDTH * 2) {
        constants.push(vec![FieldElement::ZERO; CYCLE_LENGTH]);
    }

    #[allow(clippy::needless_range_loop)]
    for i in 0..CYCLE_LENGTH {
        for j in 0..(STATE_WIDTH * 2) {
            constants[j][i] = ARK[i][j];
        }
    }

    constants
}

// HELPER FUNCTIONS
// ================================================================================================

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn add_constants(state: &mut [FieldElement], ark: &[FieldElement], offset: usize) {
    for i in 0..STATE_WIDTH {
        state[i] = state[i] + ark[offset + i];
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_sbox(state: &mut [FieldElement]) {
    for i in 0..STATE_WIDTH {
        state[i] = FieldElement::exp(state[i], ALPHA);
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_inv_sbox(state: &mut [FieldElement]) {
    // TODO: optimize
    for i in 0..STATE_WIDTH {
        state[i] = FieldElement::exp(state[i], INV_ALPHA);
    }
}

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_mds(state: &mut [FieldElement]) {
    let mut result = [FieldElement::ZERO; STATE_WIDTH];
    let mut temp = [FieldElement::ZERO; STATE_WIDTH];
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

#[inline(always)]
#[allow(clippy::needless_range_loop)]
fn apply_inv_mds(state: &mut [FieldElement]) {
    let mut result = [FieldElement::ZERO; STATE_WIDTH];
    let mut temp = [FieldElement::ZERO; STATE_WIDTH];
    for i in 0..STATE_WIDTH {
        for j in 0..STATE_WIDTH {
            temp[j] = INV_MDS[i * STATE_WIDTH + j] * state[j];
        }

        for j in 0..STATE_WIDTH {
            result[i] = result[i] + temp[j];
        }
    }
    state.copy_from_slice(&result);
}

// RESCUE CONSTANTS
// ================================================================================================
const ALPHA: u128 = 3;
const INV_ALPHA: u128 = 226854911280625642308916371969163307691;

const MDS: [FieldElement; STATE_WIDTH * STATE_WIDTH] = [
    FieldElement::new(340282366920938463463374557953744960808),
    FieldElement::new(1080),
    FieldElement::new(340282366920938463463374557953744961147),
    FieldElement::new(40),
    FieldElement::new(340282366920938463463374557953744932377),
    FieldElement::new(42471),
    FieldElement::new(340282366920938463463374557953744947017),
    FieldElement::new(1210),
    FieldElement::new(340282366920938463463374557953744079447),
    FieldElement::new(1277640),
    FieldElement::new(340282366920938463463374557953744532108),
    FieldElement::new(33880),
    FieldElement::new(340282366920938463463374557953720263017),
    FieldElement::new(35708310),
    FieldElement::new(340282366920938463463374557953733025977),
    FieldElement::new(925771),
];

const INV_MDS: [FieldElement; STATE_WIDTH * STATE_WIDTH] = [
    FieldElement::new(18020639985667067681479625318803400939),
    FieldElement::new(119196285838491236328880430704594968577),
    FieldElement::new(231409255903369280423951003551679307334),
    FieldElement::new(311938552114349342492438056332412246225),
    FieldElement::new(245698978747161380010236204726851770228),
    FieldElement::new(32113671753878130773768090116517402309),
    FieldElement::new(284248318938217584166130208504515171073),
    FieldElement::new(118503764402619831976614612559605579465),
    FieldElement::new(42476948408512208745085164298752800413),
    FieldElement::new(283594571303717652525183978492772054516),
    FieldElement::new(94047455979774690913009073579656179991),
    FieldElement::new(260445758149872374743470899536308888155),
    FieldElement::new(12603050626701424572717576220509072651),
    FieldElement::new(250660673575506110946271793719013778251),
    FieldElement::new(113894235293153614657151429548304212092),
    FieldElement::new(303406774346515776750608316419662860081),
];

pub const ARK: [[FieldElement; STATE_WIDTH * 2]; CYCLE_LENGTH] = [
    [
        FieldElement::new(252629594110556276281235816992330349983),
        FieldElement::new(121163867507455621442731872354015891839),
        FieldElement::new(244623479936175870778515556108748234900),
        FieldElement::new(181999122442017949289616572388308120964),
        FieldElement::new(130035663054758320517176088024859935575),
        FieldElement::new(274932696133623013607933255959111946013),
        FieldElement::new(130096286077538976127585373664362805864),
        FieldElement::new(209506446014122131232133742654202790201),
    ],
    [
        FieldElement::new(51912929769931267810162308005565017268),
        FieldElement::new(202610584823002946089528994694473145326),
        FieldElement::new(295992101426532309592836871256175669136),
        FieldElement::new(313404555247438968545340310449654540090),
        FieldElement::new(137671644572045862038757754124537020379),
        FieldElement::new(29113322527929260506148183779738829778),
        FieldElement::new(98634637270536166954048957710629281939),
        FieldElement::new(90484051915535813802492401077197602516),
    ],
    [
        FieldElement::new(193753019093186599897082621380539177732),
        FieldElement::new(88328997664086495053801384396180288832),
        FieldElement::new(134379598544046716907663161480793367313),
        FieldElement::new(50911186425769400405474055284903795891),
        FieldElement::new(12945394282446072785093894845750344239),
        FieldElement::new(110650301505380365788620562912149942995),
        FieldElement::new(154214463184362737046953674082326221874),
        FieldElement::new(306646039504788072647764955304698381135),
    ],
    [
        FieldElement::new(279745705918489041552127329708931301079),
        FieldElement::new(111293612078035530300709391234153848359),
        FieldElement::new(18110020378502034462498434861690576309),
        FieldElement::new(41797883582559360517115865611622162330),
        FieldElement::new(333888808893608021579859508112201825908),
        FieldElement::new(291192643991850989562610634125476905625),
        FieldElement::new(115042354025120848770557866862388897952),
        FieldElement::new(281483497320099569269754505499721335457),
    ],
    [
        FieldElement::new(172898111753678285350206449646444309824),
        FieldElement::new(202661860135906394577472615378659980424),
        FieldElement::new(141885268042225970011312316000526746741),
        FieldElement::new(270195331267041521741794476882482499817),
        FieldElement::new(196457080224171120865903216527675657315),
        FieldElement::new(56730777565482395039564396246195716949),
        FieldElement::new(4886253806084919544862202000090732791),
        FieldElement::new(147384194551383352824518757380733021990),
    ],
    [
        FieldElement::new(119476237236248181092343711369608370324),
        FieldElement::new(182869361251406039022577235058473348729),
        FieldElement::new(45308522364899994411952744852450066909),
        FieldElement::new(15438528253368638146901598290564135576),
        FieldElement::new(130060283207960095436997328133261743365),
        FieldElement::new(83953475955438079154228277940680487556),
        FieldElement::new(328659226769709797512044291035930357326),
        FieldElement::new(228749522131871685132212950281473676382),
    ],
    [
        FieldElement::new(46194972462682851176957413491161426658),
        FieldElement::new(296333983305826854863835978241833143471),
        FieldElement::new(138957733159616849361016139528307260698),
        FieldElement::new(67842086763518777676559492559456199109),
        FieldElement::new(45580040156133202522383315452912604930),
        FieldElement::new(67567837934606680937620346425373752595),
        FieldElement::new(202860989528104560171546683198384659325),
        FieldElement::new(22630500510153322451285114937258973361),
    ],
    [
        FieldElement::new(324160761097464842200838878419866223614),
        FieldElement::new(338466547889555546143667391979278153877),
        FieldElement::new(189171173535649401433078628567098769571),
        FieldElement::new(162173266902020502126600904559755837464),
        FieldElement::new(136209703129442038834374731074825683052),
        FieldElement::new(61998071517031804812562190829480056772),
        FieldElement::new(307309080039351604461536918194634835054),
        FieldElement::new(26708622949278137915061761772299784349),
    ],
    [
        FieldElement::new(129516553661717764361826568456881002617),
        FieldElement::new(224023580754958002183324313900177991825),
        FieldElement::new(17590440203644538688189654586240082513),
        FieldElement::new(135610063062379124269847491297867667710),
        FieldElement::new(146865534517067293442442506551295645352),
        FieldElement::new(238139104484181583196227119098779158429),
        FieldElement::new(39300761479713744892853256947725570060),
        FieldElement::new(54114440355764484955231402374312070440),
    ],
    [
        FieldElement::new(222758070305343916663075833184045878425),
        FieldElement::new(323840793618712078836672915700599856701),
        FieldElement::new(103586087979277053032666296091805459741),
        FieldElement::new(160263698024385270625527195046420579470),
        FieldElement::new(76620453913654705501329735586535761337),
        FieldElement::new(117793948142462197480091377165008040465),
        FieldElement::new(86998218841589258723143213495722487114),
        FieldElement::new(203188618662906890442620821687773659689),
    ],
    [
        FieldElement::new(313098786815741054633864043424353402357),
        FieldElement::new(133085673687338880872979866135939079867),
        FieldElement::new(219888424885634764555580944265544343421),
        FieldElement::new(5893221169005427793512575133564978746),
        FieldElement::new(123830602624063632344313821515642988189),
        FieldElement::new(99030942908036387138287682010525589136),
        FieldElement::new(181549003357535890945363082242256699137),
        FieldElement::new(152424978799328476472358562493335008209),
    ],
    [
        FieldElement::new(274481943862544603168725464029979191673),
        FieldElement::new(4975004592976331754728718693838357226),
        FieldElement::new(101850445399221640701542169338886750079),
        FieldElement::new(230325699922192981509673754024218912397),
        FieldElement::new(50419227750575087142720761582056939006),
        FieldElement::new(112444234528764731925178653200320603078),
        FieldElement::new(312169855609816651638877239277948636598),
        FieldElement::new(204255114617024487729019111502542629940),
    ],
    [
        FieldElement::new(95797476952346525817251811755749179939),
        FieldElement::new(306977388944722094681694167558392710189),
        FieldElement::new(300754874465668732709232449646112602172),
        FieldElement::new(25567836410351071106804347269705784680),
        FieldElement::new(129659188855548935155840545784705385753),
        FieldElement::new(228441586459539470069565041053012869566),
        FieldElement::new(178382533299631576605259357906020320778),
        FieldElement::new(274458637266680353971597477639962034316),
    ],
    [
        FieldElement::new(280059913840028448065185235205261648486),
        FieldElement::new(246537412674731137211182698562269717969),
        FieldElement::new(259930078572522349821084822750913159564),
        FieldElement::new(186061633995391650657311511040160727356),
        FieldElement::new(179777566992900315528995607912777709520),
        FieldElement::new(209753365793154515863736129686836743468),
        FieldElement::new(270445008049478596978645420017585428243),
        FieldElement::new(70998387591825316724846035292940615733),
    ],
    [FieldElement::ZERO; 8],
    [FieldElement::ZERO; 8],
];
