use crate::{pond::Matter, CoreError};
use serde::{Deserialize, Serialize};

const MAX_DEPOSITS: usize = 16;
const VISUAL_RETIRE_MG_C: f64 = 0.01;
const VISUAL_RETIRE_FRACTION: f64 = 0.001;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondLeafDepositPreset {
    Midground,
    Foreground,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondLeafDepositStage {
    Whole,
    Fragments,
    Fine,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondLeafDepositObservation {
    pub id: u32,
    pub x: f64,
    pub preset: PondLeafDepositPreset,
    pub initial_mg_c: f64,
    pub remaining_mg_c: f64,
    pub stage: PondLeafDepositStage,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeafDeposit {
    id: u32,
    x: f64,
    preset: PondLeafDepositPreset,
    initial_mg_c: f64,
    remaining_mg_c: f64,
}

impl LeafDeposit {
    fn observe(self) -> PondLeafDepositObservation {
        let fraction = self.remaining_mg_c / self.initial_mg_c;
        let stage = if fraction >= 0.60 { PondLeafDepositStage::Whole }
            else if fraction >= 0.20 { PondLeafDepositStage::Fragments }
            else { PondLeafDepositStage::Fine };
        PondLeafDepositObservation {
            id: self.id, x: self.x, preset: self.preset, initial_mg_c: self.initial_mg_c,
            remaining_mg_c: self.remaining_mg_c, stage,
        }
    }
}

/// Carbon attribution for a displayed deposit. The existing detritus pool is
/// the only stock used for reactions or the basin inventory.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondLeafLayout {
    next_id: u32,
    deposits: Vec<LeafDeposit>,
}

impl PondLeafLayout {
    pub(super) fn new() -> Self { Self { next_id: 1, deposits: Vec::new() } }

    pub(super) fn add(layout: &mut Option<Self>, x: f64, preset: PondLeafDepositPreset, mg_c: f64) -> Result<Matter, CoreError> {
        if !valid_dose(mg_c) { return Err(CoreError::InvalidPondEdit); }
        let dose = detritus_matter(mg_c);
        if let Some(layout) = layout {
            layout.place(x, preset, dose)?;
        } else {
            let mut new = Self::new();
            new.place(x, preset, dose)?;
            *layout = Some(new);
        }
        Ok(dose)
    }

    pub(super) fn has_deposits(&self) -> bool { !self.deposits.is_empty() }

    pub(super) fn observe(&self) -> Option<Vec<PondLeafDepositObservation>> {
        self.has_deposits().then(|| self.deposits.iter().copied().map(LeafDeposit::observe).collect())
    }

    pub(super) fn located_carbon(&self) -> f64 {
        self.deposits.iter().map(|deposit| deposit.remaining_mg_c).sum()
    }

    pub(super) fn place(&mut self, x: f64, preset: PondLeafDepositPreset, dose: Matter) -> Result<(), CoreError> {
        check_site(x)?;
        if self.deposits.len() >= MAX_DEPOSITS { return Err(CoreError::PondLeafDepositLimit); }
        let next_id = self.next_id.checked_add(1).ok_or(CoreError::PondLeafDepositLimit)?;
        self.deposits.push(LeafDeposit { id: self.next_id, x, preset, initial_mg_c: dose.carbon(), remaining_mg_c: dose.carbon() });
        self.next_id = next_id;
        Ok(())
    }

    pub(super) fn move_deposit(&mut self, id: u32, x: f64) -> Result<(), CoreError> {
        check_site(x)?;
        let deposit = self.deposits.iter_mut().find(|deposit| deposit.id == id).ok_or(CoreError::PondLeafDepositMissing)?;
        deposit.x = x;
        Ok(())
    }

    pub(super) fn remove_deposit(&mut self, id: u32, pool: &mut Matter) -> Result<Matter, CoreError> {
        let index = self.deposits.iter().position(|deposit| deposit.id == id)
            .ok_or(CoreError::PondLeafDepositMissing)?;
        let deposit = self.deposits.remove(index);
        // The marker owns a carbon attribution. Its nitrogen and phosphorus share
        // the current pool composition, just as they do when the pond is split.
        Ok(pool.take_carbon(deposit.remaining_mg_c))
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    pub(super) fn preserve_optional_allocations(prior: &mut Option<Self>, current: Option<&Self>) {
        if let Some(current) = current {
            prior.get_or_insert_with(Self::new).preserve_allocations(current);
        }
    }

    pub(super) fn reset_stocks(&mut self) -> impl Iterator<Item = Matter> + '_ {
        self.deposits.iter_mut().map(|deposit| {
            deposit.remaining_mg_c = deposit.initial_mg_c;
            detritus_matter(deposit.initial_mg_c)
        })
    }

    pub(super) fn decay(&mut self, before: f64, after: f64) {
        if before <= after { return; }
        let retained_fraction = after / before;
        self.deposits.retain_mut(|deposit| {
            deposit.remaining_mg_c *= retained_fraction;
            // Subvisual material remains in the shared pool after its marker disappears.
            let retire = VISUAL_RETIRE_MG_C.min(deposit.initial_mg_c * VISUAL_RETIRE_FRACTION);
            deposit.remaining_mg_c > retire
        });
    }

    pub(super) fn valid(&self, pool_c: f64) -> bool {
        let mut last_id = 0;
        let mut attributed_c = 0.0;
        if self.next_id == 0 || self.deposits.len() > MAX_DEPOSITS { return false; }
        for deposit in &self.deposits {
            if !(last_id < deposit.id && deposit.id < self.next_id && valid_dose(deposit.initial_mg_c)
                && 0.0 < deposit.remaining_mg_c && deposit.remaining_mg_c <= deposit.initial_mg_c && check_site(deposit.x).is_ok()) {
                return false;
            }
            last_id = deposit.id;
            attributed_c += deposit.remaining_mg_c;
        }
        Self::fits_pool(attributed_c, pool_c)
    }

    pub(super) fn fits_pool(attributed_c: f64, pool_c: f64) -> bool {
        if pool_c == 0.0 { return attributed_c == 0.0; }
        attributed_c <= pool_c + 1e-8 + 1e-12 * pool_c
    }
}

pub(super) fn valid_dose(mg_c: f64) -> bool { 0.0 < mg_c && mg_c <= 50_000.0 }

pub(super) fn detritus_matter(mg_c: f64) -> Matter { Matter::with_ratios(mg_c, 0.034, 0.005) }

fn check_site(x: f64) -> Result<(), CoreError> {
    if (0.08..=0.92).contains(&x) { Ok(()) } else { Err(CoreError::InvalidPondLeafDepositSite) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r5_pond_leaf_deposit_stages_report_retained_carbon_and_retire_visuals_only() {
        let mut layout = PondLeafLayout::new();
        layout.place(0.32, PondLeafDepositPreset::Midground, detritus_matter(10.0)).unwrap();
        assert_eq!(layout.observe().unwrap()[0].stage, PondLeafDepositStage::Whole);
        layout.decay(50.0, 25.0);
        assert_eq!(layout.observe().unwrap()[0].remaining_mg_c, 5.0);
        assert_eq!(layout.observe().unwrap()[0].stage, PondLeafDepositStage::Fragments);
        layout.decay(25.0, 9.0);
        assert!((layout.observe().unwrap()[0].remaining_mg_c - 1.8).abs() < 1e-12);
        assert_eq!(layout.observe().unwrap()[0].stage, PondLeafDepositStage::Fine);
        layout.decay(9.0, 0.001);
        assert!(layout.observe().is_none());
        assert!(layout.valid(0.001));
        layout.place(0.32, PondLeafDepositPreset::Foreground, detritus_matter(20.0)).unwrap();
        assert_eq!(layout.observe().unwrap()[0].id, 2);
    }
}
