//! Mythic GME oracle — Fate Chart checks, Chaos Factor, Scene Checks, Random
//! Events.
//!
//! Ported from `src/harsh_realm/engine/oracle.py`. The deterministic
//! classifications ([`classify_fate`], [`classify_scene`],
//! [`roll_on_ranged_table`]) are the pure core; the [`FateChecker`],
//! [`ChaosTracker`], [`SceneChecker`], and [`RandomEventGenerator`] add the
//! RNG-driven rolls and content loading on top.

use rand::Rng;

use crate::oracle_models::{
    FateChartDocument, FateCheckResult, FateResult, OracleEventTable, OracleTableEntry, RandomEvent,
    SceneCheckResult, SceneModification,
};
use crate::packs::paths::default_content_dir;

/// Classify a fate-chart d100 `roll` against its thresholds.
///
/// Mirrors `FateChartChecker.check`: exceptional-yes if `roll <= exc_yes`, else
/// yes if `roll <= yes_threshold`, else exceptional-no if `roll >= exc_no`, else
/// no. Returns the `FateResult` value string.
pub fn classify_fate(roll: i64, yes_threshold: i64, exc_yes: i64, exc_no: i64) -> &'static str {
    if roll <= exc_yes {
        "exceptional_yes"
    } else if roll <= yes_threshold {
        "yes"
    } else if roll >= exc_no {
        "exceptional_no"
    } else {
        "no"
    }
}

/// Classify a scene-check d10 `roll` against the chaos factor.
///
/// Mirrors `SceneChecker.check` (chaos clamped to 1..=9): normal if
/// `roll > chaos`, else interrupt on odd rolls, else altered. Returns the
/// `SceneModification` value string.
pub fn classify_scene(roll: i64, chaos_factor: i64) -> &'static str {
    let chaos = chaos_factor.clamp(1, 9);
    if roll > chaos {
        "normal"
    } else if roll % 2 == 1 {
        "interrupt"
    } else {
        "altered"
    }
}

/// Index of the first event-table entry whose inclusive range contains `roll`
/// (`_roll_on_table`).
pub fn roll_on_ranged_table(ranges: &[(i64, i64)], roll: i64) -> Option<usize> {
    ranges
        .iter()
        .position(|(lo, hi)| *lo <= roll && roll <= *hi)
}

/// Map a [`FateResult`] to its narration label.
fn fate_result_label(result: FateResult) -> &'static str {
    match result {
        FateResult::ExceptionalYes => "Exceptional Yes!",
        FateResult::Yes => "Yes",
        FateResult::No => "No",
        FateResult::ExceptionalNo => "Exceptional No!",
    }
}

fn fate_result_from_key(key: &str) -> FateResult {
    match key {
        "exceptional_yes" => FateResult::ExceptionalYes,
        "yes" => FateResult::Yes,
        "exceptional_no" => FateResult::ExceptionalNo,
        _ => FateResult::No,
    }
}

/// Format a fate check result as narration text.
pub fn format_fate_narration(
    likelihood: &str,
    chaos_factor: i32,
    roll: i32,
    yes_threshold: i32,
    result: FateResult,
) -> String {
    format!(
        "Fate check ({likelihood}, chaos {chaos_factor}): **{}** (rolled {roll} vs threshold {yes_threshold})",
        fate_result_label(result)
    )
}

fn oracle_dir() -> std::path::PathBuf {
    default_content_dir().join("tables").join("oracle")
}

/// Load the fate chart document from content.
pub fn load_chart() -> Result<FateChartDocument, String> {
    let path = oracle_dir().join("fate_chart.yaml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read fate chart {}: {e}", path.display()))?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

/// Load an oracle event table document by name from content.
pub fn load_event_table(name: &str) -> Result<OracleEventTable, String> {
    let path = oracle_dir().join(format!("{name}.yaml"));
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read event table {}: {e}", path.display()))?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

/// Find the entry matching a d100 roll in a ranged table (with last-entry
/// fallback, then a generic default).
fn roll_on_event_table(table: &OracleEventTable, roll: i32) -> OracleTableEntry {
    if let Some(entry) = table
        .entries
        .iter()
        .find(|e| e.min <= roll && roll <= e.max)
    {
        return entry.clone();
    }
    table.entries.last().cloned().unwrap_or(OracleTableEntry {
        min: 1,
        max: 100,
        focus: Some("Unknown".to_string()),
        action: Some("Unknown".to_string()),
        subject: Some("Unknown".to_string()),
        description: String::new(),
        extra: crate::runtime::JsonObject::new(),
    })
}

/// Resolves fate chart checks using the Mythic GME system, caching the chart.
#[derive(Default)]
pub struct FateChecker {
    chart: Option<FateChartDocument>,
}

impl FateChecker {
    /// Create a fate checker (loads the chart lazily on first check).
    pub fn new() -> Self {
        FateChecker { chart: None }
    }

    fn ensure_chart(&mut self) -> Result<&FateChartDocument, String> {
        if self.chart.is_none() {
            self.chart = Some(load_chart()?);
        }
        Ok(self.chart.as_ref().expect("just populated"))
    }

    /// Perform a fate chart check for a likelihood key and chaos factor.
    pub fn check<R: Rng>(
        &mut self,
        likelihood: &str,
        chaos_factor: i32,
        rng: &mut R,
    ) -> Result<FateCheckResult, String> {
        let chaos = chaos_factor.clamp(1, 9);
        let chart = self.ensure_chart()?;
        let row = chart
            .likelihoods
            .get_likelihood(likelihood)
            .ok_or_else(|| format!("Unknown likelihood: {likelihood:?}"))?;
        let thresholds = row
            .for_chaos(chaos)
            .ok_or_else(|| format!("Invalid chaos factor: {chaos}"))?;

        let roll = rng.gen_range(1..=100);
        let key = classify_fate(
            roll as i64,
            thresholds.yes_threshold as i64,
            thresholds.exceptional_yes as i64,
            thresholds.exceptional_no as i64,
        );
        let result = fate_result_from_key(key);
        let narration =
            format_fate_narration(likelihood, chaos, roll, thresholds.yes_threshold, result);

        Ok(FateCheckResult {
            likelihood: likelihood.to_string(),
            chaos_factor: chaos,
            roll,
            result,
            narration,
        })
    }
}

/// Tracks and modifies the Mythic GME chaos factor (1-9).
///
/// An optional on-change callback receives `(old, new)` whenever the factor
/// changes; callers use it to emit `oracle.chaos_changed` events.
pub struct ChaosTracker {
    chaos: i32,
    on_change: Option<Box<dyn FnMut(i32, i32)>>,
}

impl ChaosTracker {
    /// Create a tracker with an initial chaos factor (clamped to 1..=9).
    pub fn new(initial: i32) -> Self {
        ChaosTracker {
            chaos: initial.clamp(1, 9),
            on_change: None,
        }
    }

    /// Attach an on-change callback.
    pub fn with_on_change(mut self, on_change: Box<dyn FnMut(i32, i32)>) -> Self {
        self.on_change = Some(on_change);
        self
    }

    /// Current chaos factor.
    pub fn value(&self) -> i32 {
        self.chaos
    }

    fn apply(&mut self, new_value: i32) -> i32 {
        let old = self.chaos;
        self.chaos = new_value.clamp(1, 9);
        if self.chaos != old {
            if let Some(cb) = self.on_change.as_mut() {
                cb(old, self.chaos);
            }
        }
        self.chaos
    }

    /// Increase chaos by 1, clamped at 9. Returns the new value.
    pub fn increase(&mut self) -> i32 {
        self.apply(self.chaos + 1)
    }

    /// Decrease chaos by 1, clamped at 1. Returns the new value.
    pub fn decrease(&mut self) -> i32 {
        self.apply(self.chaos - 1)
    }

    /// Set chaos to a value, clamped to 1..=9. Returns the new value.
    pub fn set(&mut self, value: i32) -> i32 {
        self.apply(value)
    }
}

/// Performs scene checks at the start of new scenes.
#[derive(Default)]
pub struct SceneChecker;

impl SceneChecker {
    /// Perform a scene check against the chaos factor.
    pub fn check<R: Rng>(&self, chaos_factor: i32, rng: &mut R) -> SceneCheckResult {
        let chaos = chaos_factor.clamp(1, 9);
        let roll = rng.gen_range(1..=10);
        let key = classify_scene(roll as i64, chaos as i64);
        let (modification, narration) = match key {
            "normal" => (
                SceneModification::Normal,
                format!("Scene check (d10={roll} vs chaos {chaos}): Scene proceeds normally."),
            ),
            "interrupt" => (
                SceneModification::Interrupt,
                format!("Scene check (d10={roll} vs chaos {chaos}): **Interrupt!** A random event occurs."),
            ),
            _ => (
                SceneModification::Altered,
                format!("Scene check (d10={roll} vs chaos {chaos}): Scene is **altered** from expectations."),
            ),
        };
        SceneCheckResult {
            roll,
            chaos_factor: chaos,
            modification,
            narration,
        }
    }
}

/// Generates random events from the Focus x Action x Subject tables.
#[derive(Default)]
pub struct RandomEventGenerator;

impl RandomEventGenerator {
    /// Generate a random event by rolling on the three oracle event tables.
    pub fn generate<R: Rng>(&self, rng: &mut R) -> Result<RandomEvent, String> {
        let focus_table = load_event_table("event_focus")?;
        let action_table = load_event_table("event_action")?;
        let subject_table = load_event_table("event_subject")?;

        let focus_roll = rng.gen_range(1..=100);
        let action_roll = rng.gen_range(1..=100);
        let subject_roll = rng.gen_range(1..=100);

        let focus_entry = roll_on_event_table(&focus_table, focus_roll);
        let action_entry = roll_on_event_table(&action_table, action_roll);
        let subject_entry = roll_on_event_table(&subject_table, subject_roll);

        let focus = focus_entry.focus.clone().unwrap_or_else(|| "Unknown".into());
        let action = action_entry.action.clone().unwrap_or_else(|| "Unknown".into());
        let subject = subject_entry
            .subject
            .clone()
            .unwrap_or_else(|| "Unknown".into());

        let narration = format!(
            "Random event: **{focus}** — {action} / {subject}. ({})",
            focus_entry.description
        );

        Ok(RandomEvent {
            focus,
            action,
            subject,
            focus_roll,
            action_roll,
            subject_roll,
            narration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn chaos_tracker_clamps_and_notifies() {
        let mut tracker = ChaosTracker::new(20); // clamps to 9
        assert_eq!(tracker.value(), 9);
        assert_eq!(tracker.increase(), 9); // already max
        assert_eq!(tracker.set(5), 5);
        assert_eq!(tracker.decrease(), 4);

        let mut changes: Vec<(i32, i32)> = Vec::new();
        // Use a tracker whose callback records transitions.
        let recorded = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = recorded.clone();
        let mut t2 = ChaosTracker::new(5)
            .with_on_change(Box::new(move |old, new| sink.borrow_mut().push((old, new))));
        t2.increase();
        t2.decrease();
        changes.extend(recorded.borrow().iter().copied());
        assert_eq!(changes, vec![(5, 6), (6, 5)]);
    }

    #[test]
    fn scene_checker_matches_classification() {
        let checker = SceneChecker;
        let mut rng = SmallRng::seed_from_u64(1);
        let result = checker.check(5, &mut rng);
        assert!(result.roll >= 1 && result.roll <= 10);
        assert_eq!(result.chaos_factor, 5);
        // Narration should reflect the modification branch.
        let expected = match result.modification {
            SceneModification::Normal => "proceeds normally",
            SceneModification::Interrupt => "Interrupt!",
            SceneModification::Altered => "altered",
        };
        assert!(result.narration.contains(expected));
    }

    #[test]
    fn fate_result_labels() {
        assert_eq!(fate_result_label(FateResult::ExceptionalYes), "Exceptional Yes!");
        assert_eq!(fate_result_from_key("yes"), FateResult::Yes);
        assert_eq!(fate_result_from_key("garbage"), FateResult::No);
        let n = format_fate_narration("likely", 5, 12, 75, FateResult::Yes);
        assert!(n.contains("Fate check (likely, chaos 5)"));
        assert!(n.contains("**Yes**"));
    }

    #[test]
    fn fate_classification_bands() {
        // yes=50, exc_yes=10, exc_no=91
        assert_eq!(classify_fate(5, 50, 10, 91), "exceptional_yes");
        assert_eq!(classify_fate(10, 50, 10, 91), "exceptional_yes");
        assert_eq!(classify_fate(11, 50, 10, 91), "yes");
        assert_eq!(classify_fate(50, 50, 10, 91), "yes");
        assert_eq!(classify_fate(51, 50, 10, 91), "no");
        assert_eq!(classify_fate(90, 50, 10, 91), "no");
        assert_eq!(classify_fate(91, 50, 10, 91), "exceptional_no");
        assert_eq!(classify_fate(100, 50, 10, 91), "exceptional_no");
    }

    #[test]
    fn scene_classification() {
        // chaos 5
        assert_eq!(classify_scene(6, 5), "normal"); // roll > chaos
        assert_eq!(classify_scene(10, 5), "normal");
        assert_eq!(classify_scene(5, 5), "interrupt"); // <= chaos, odd
        assert_eq!(classify_scene(3, 5), "interrupt");
        assert_eq!(classify_scene(4, 5), "altered"); // <= chaos, even
    }

    #[test]
    fn scene_chaos_clamped_low() {
        // chaos 0 clamps to 1: roll 1 → not > 1, odd → interrupt
        assert_eq!(classify_scene(1, 0), "interrupt");
        assert_eq!(classify_scene(2, 0), "normal"); // 2 > 1
    }

    #[test]
    fn event_table_lookup() {
        let ranges = [(1, 7), (8, 28), (29, 71)];
        assert_eq!(roll_on_ranged_table(&ranges, 1), Some(0));
        assert_eq!(roll_on_ranged_table(&ranges, 28), Some(1));
        assert_eq!(roll_on_ranged_table(&ranges, 71), Some(2));
        assert_eq!(roll_on_ranged_table(&ranges, 99), None);
    }
}
