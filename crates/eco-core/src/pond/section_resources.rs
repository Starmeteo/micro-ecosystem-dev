use super::*;

pub(super) struct AttachedGrowth {
    moss: [Matter; MAX_MOSS_PATCHES],
    film: [Matter; MAX_STONES],
}

impl AttachedGrowth {
    pub(super) fn accumulate(&self, total: &mut Matter) {
        for request in self.film.into_iter().chain(self.moss) { total.add(request); }
    }
}

pub(super) struct SectionRespiration {
    moss: [Option<f64>; MAX_MOSS_PATCHES],
    film: [f64; MAX_STONES],
    shrimp: [f64; MAX_SHRIMP],
}

impl SectionRespiration {
    pub(super) fn total(&self) -> f64 {
        self.film.into_iter().chain(self.moss.into_iter().flatten()).chain(self.shrimp).sum()
    }
}

#[derive(Default)]
pub(super) struct SectionRespiratoryFlux {
    pub(super) producer_fuel: Matter,
    pub(super) animal_fuel: Matter,
    pub(super) shed: Matter,
}

impl PondStaticSection {
    fn wet_moss(&self, patch: &pond_moss::PondMossPatch, column_px: f64) -> bool {
        pond_moss::support(patch.parent, patch.offset, Some(&self.stones), Some(&self.woods))
            .is_ok_and(|support| support.submerged(column_px))
    }

    pub(super) fn attached_growth_demand(&self, column_px: f64, opportunity: f64) -> AttachedGrowth {
        let mut demand = AttachedGrowth { moss: [Matter::default(); MAX_MOSS_PATCHES], film: [Matter::default(); MAX_STONES] };
        let film = self.stone_biofilm.iter().flatten();
        for ((patch, wet), request) in film.zip(self.stone_biofilm_wet_fractions(column_px)).zip(&mut demand.film) {
            *request = patch.growth_demand(wet, opportunity);
        }
        let moss = self.moss_layout.iter().flat_map(|layout| &layout.patches);
        for (patch, request) in moss.zip(&mut demand.moss) {
            if self.wet_moss(patch, column_px) { *request = patch.growth_demand(opportunity); }
        }
        demand
    }

    pub(super) fn apply_attached_growth(&mut self, demand: AttachedGrowth, supply: f64) {
        for (patch, request) in self.stone_biofilm.iter_mut().flatten().zip(demand.film) {
            patch.stock.add(request.scaled(supply));
        }
        for (patch, request) in self.moss_layout.iter_mut().flat_map(|layout| &mut layout.patches).zip(demand.moss) {
            patch.stock_mg_cnp.add(request.scaled(supply));
        }
    }

    pub(super) fn extended_respiration_demand(&self, column_px: f64, elapsed: f64) -> SectionRespiration {
        let mut demand = SectionRespiration { moss: [None; MAX_MOSS_PATCHES], film: [0.0; MAX_STONES], shrimp: [0.0; MAX_SHRIMP] };
        let film = self.stone_biofilm.iter().flatten();
        for ((patch, wet), request) in film.zip(self.stone_biofilm_wet_fractions(column_px)).zip(&mut demand.film) {
            *request = patch.respiration_demand(wet, elapsed);
        }
        let moss = self.moss_layout.iter().flat_map(|layout| &layout.patches);
        for (patch, request) in moss.zip(&mut demand.moss) {
            *request = self.wet_moss(patch, column_px).then(|| patch.respiration_demand(elapsed));
        }
        for (individual, request) in self.shrimp.iter().zip(&mut demand.shrimp) {
            *request = (individual.body.carbon() * 0.019 * elapsed).min(individual.reserve.carbon());
        }
        demand
    }

    pub(super) fn apply_extended_respiration(&mut self, demand: SectionRespiration, supply: f64, elapsed: f64) -> SectionRespiratoryFlux {
        let mut realized = SectionRespiratoryFlux::default();
        for (patch, request) in self.stone_biofilm.iter_mut().flatten().zip(demand.film) {
            realized.producer_fuel.add(patch.stock.take_carbon(request * supply));
        }
        for (patch, request) in self.moss_layout.iter_mut().flat_map(|layout| &mut layout.patches).zip(demand.moss) {
            if let Some(request) = request {
                realized.producer_fuel.add(patch.stock_mg_cnp.take_carbon(request * supply));
                realized.shed.add(patch.senesce(elapsed));
            }
        }
        for (individual, request) in self.shrimp.iter_mut().zip(demand.shrimp) {
            realized.animal_fuel.add(individual.reserve.take_carbon(request * supply));
        }
        realized
    }
}
