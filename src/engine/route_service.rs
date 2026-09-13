//! Route cost calculation service.

use crate::data::*;
use crate::engine::GameEngine;
use crate::state::{GameState, PlayerStats};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Calculated route costs
#[derive(Debug, Clone)]
pub struct RouteCosts {
    pub fuel: u32,
    pub time: u32,
    pub risk: u32,
}

/// Route calculation service
pub struct RouteService;

impl RouteService {
    /// The cost of a route as the game will actually charge it.
    ///
    /// The driving screen used to print the raw `GAME_CONSTANTS` base costs
    /// while the engine charged the modified ones — weather, hazards, route
    /// mastery, curse pressure and skills all move the figure — so a player
    /// could read "25 min", pick it with thirty on the clock, and lose the
    /// shift to "Not enough time for this route". Both sides now come through
    /// here, so the quoted price is the price.
    ///
    /// Curse and streak pressure are deliberately excluded: both mutate
    /// `GameState` when applied, and a screen must not change the game by
    /// drawing it. They can only push the cost up, so the quote is a floor.
    pub fn quote_route(
        route: RouteType,
        state: &GameState,
        data: &GameData,
        stats: &PlayerStats,
    ) -> RouteCosts {
        let mut passenger_risk = state
            .current_passenger
            .as_ref()
            .and_then(|p| data.get_location(&p.pickup).map(|l| l.risk_level))
            .unwrap_or(1);
        if let Some(passenger) = state.current_passenger.as_ref() {
            if let Some(reputation) = state.passenger_reputation.get(&passenger.id) {
                let adjusted =
                    passenger_risk as i32 + reputation.risk_modifier(&data.constants.reputation);
                passenger_risk =
                    adjusted.clamp(0, data.constants.risk.max_risk_level as i32) as u32;
            }
        }

        let mut costs = Self::calculate_route_costs(
            route,
            &data.constants,
            passenger_risk,
            Some(&state.current_weather),
            Some(&state.time_of_day),
            &state.environmental_hazards,
            &stats.route_mastery_map(),
            state.current_passenger.as_ref(),
            &crate::engine::SkillModifiers::from_unlocked(&data.skills, &stats.unlocked_skills),
        );
        Self::apply_location_modifiers(&mut costs, state, data);
        Self::apply_headlight_modifier(&mut costs, state);
        costs
    }

    /// What a passenger would pay on each road, smallest and largest.
    ///
    /// The ride offer showed `passenger.fare`, the raw authored base, while the
    /// payout runs it through the route's multiplier, the passenger's feeling
    /// about that route, the repeat-route penalty, the driver's standing with
    /// them, the destination's modifier and the fare skills. Every one of those
    /// is already known when the offer is on screen -- the destination is
    /// printed on the same card -- so the number the driver was shown could be
    /// most of a third under what they would actually earn.
    ///
    /// Both ends come from `GameEngine::calculate_fare` itself, called once per
    /// route, so the offer cannot drift from the payout. The spread is the road
    /// not yet chosen and nothing else.
    pub fn fare_range(
        passenger: &Passenger,
        state: &GameState,
        data: &GameData,
        stats: &PlayerStats,
    ) -> (u32, u32) {
        let skill_mods =
            crate::engine::SkillModifiers::from_unlocked(&data.skills, &stats.unlocked_skills);
        let destination_fare_modifier = data
            .get_location(&passenger.destination)
            .map(|location| location.fare_modifier)
            .unwrap_or(1.0)
            * skill_mods.fare_mult;
        let reputation = state.passenger_reputation.get(&passenger.id);

        let (low, high) = [
            RouteType::Normal,
            RouteType::Shortcut,
            RouteType::Scenic,
            RouteType::Police,
        ]
        .into_iter()
        .map(|route| {
            GameEngine::fare_before_variation(
                passenger.fare,
                route,
                passenger,
                state.consecutive_route_streak.as_ref(),
                reputation,
                &data.constants,
                destination_fare_modifier,
            )
        })
        .fold((f32::MAX, 0.0_f32), |(low, high), fare| {
            (low.min(fare), high.max(fare))
        });

        // Widened by the meter's wobble at both ends, so the payout always
        // lands inside the range the driver was quoted.
        (
            (low - GameEngine::FARE_VARIATION).max(GameEngine::MINIMUM_FARE) as u32,
            (high + GameEngine::FARE_VARIATION).max(GameEngine::MINIMUM_FARE) as u32,
        )
    }

    /// Calculate costs for a route type with all modifiers
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_route_costs(
        route: RouteType,
        constants: &ConstantsData,
        passenger_risk: u32,
        weather: Option<&WeatherCondition>,
        time_of_day: Option<&TimeOfDay>,
        hazards: &[EnvironmentalHazard],
        route_mastery: &HashMap<RouteType, u32>,
        passenger: Option<&Passenger>,
        skill_mods: &crate::engine::SkillModifiers,
    ) -> RouteCosts {
        // Base costs from constants
        let (base_fuel, base_time, base_risk) = match route {
            RouteType::Normal => (
                constants.game_constants.fuel_cost_normal,
                constants.game_constants.time_cost_normal,
                constants.game_constants.risk_normal,
            ),
            RouteType::Shortcut => (
                constants.game_constants.fuel_cost_shortcut,
                constants.game_constants.time_cost_shortcut,
                constants.game_constants.risk_shortcut,
            ),
            RouteType::Scenic => (
                constants.game_constants.fuel_cost_scenic,
                constants.game_constants.time_cost_scenic,
                constants.game_constants.risk_scenic,
            ),
            RouteType::Police => (
                constants.game_constants.fuel_cost_police,
                constants.game_constants.time_cost_police,
                constants.game_constants.risk_police,
            ),
        };

        // `RISK.MAX_RISK_LEVEL` is the authored ceiling. The risk maths used a
        // hardcoded 5.0 throughout while the constant said 4, so every
        // threshold expressed against it was being measured on a different
        // scale from the one the constants describe.
        let max_risk = constants.risk.max_risk_level as f32;

        let mut fuel = base_fuel as f32;
        let mut time = base_time as f32;
        let mut risk = base_risk as f32 + (passenger_risk as f32 - 1.0);

        // Apply weather effects
        if let Some(w) = weather {
            for effect in &w.effects {
                match effect.effect_type {
                    WeatherEffectType::FuelConsumption => fuel *= 1.0 + effect.value as f32 / 100.0,
                    WeatherEffectType::TimeDelay => time *= 1.0 + effect.value as f32 / 100.0,
                    WeatherEffectType::VisibilityReduction => risk += effect.value as f32 / 100.0,
                    WeatherEffectType::SupernaturalAttraction => {
                        risk += effect.value as f32 / 30.0;
                    }
                    WeatherEffectType::PassengerBehavior => {
                        if passenger.is_some() {
                            risk += effect.value as f32 / 50.0;
                            time += effect.value.max(0) as f32 / 4.0;
                        }
                    }
                    WeatherEffectType::RouteBlockage => {
                        if Self::weather_effect_applies_to_route(effect, route) {
                            time += effect.value.max(0) as f32 / 2.0;
                            risk = (risk + effect.value.max(0) as f32 / 10.0).min(max_risk);
                        }
                    }
                }
            }

            // Heavy weather penalties
            if w.intensity == WeatherIntensity::Heavy {
                if route == RouteType::Shortcut
                    && matches!(
                        w.weather_type,
                        WeatherType::Rain | WeatherType::Fog | WeatherType::Snow
                    )
                {
                    fuel += 8.0;
                    time += 10.0;
                    risk = (risk + 2.0).min(max_risk);
                }
                if route == RouteType::Scenic && w.weather_type == WeatherType::Thunderstorm {
                    fuel += 5.0;
                    time += 15.0;
                    risk = (risk + 1.0).min(max_risk);
                }
            }
        }

        // Time of day effects
        if let Some(tod) = time_of_day {
            if matches!(tod.phase, TimePhase::Night | TimePhase::Latenight) {
                risk += 0.2;
            }
            if tod.ambient_light < 30 {
                fuel *= 1.1; // Headlights
            }
            if route == RouteType::Scenic && tod.phase == TimePhase::Latenight {
                fuel += 3.0;
                time += 8.0;
                risk = (risk + 1.0).min(max_risk);
            }
        }

        // Passenger fear penalty
        if let Some(p) = passenger {
            if p.fears_route(route) {
                fuel += 2.0;
                time += 3.0;
                risk = (risk + 1.0).min(max_risk);
            }
        }

        // Hazard effects, accumulated separately so the Reinforced Chassis skill
        // (hazard_mult) can soften only the hazard-inflicted portion.
        let mut hazard_fuel = 0.0;
        let mut hazard_time = 0.0;
        let mut hazard_risk = 0.0;
        for hazard in hazards {
            if hazard.blocks_route(route) {
                hazard_time += 12.0;
                hazard_fuel += 5.0;
                hazard_risk += 2.0;
            }
            if let Some(f) = hazard.effects.fuel_increase {
                hazard_fuel += f as f32;
            }
            if let Some(t) = hazard.effects.time_delay {
                hazard_time += t as f32;
            }
            if let Some(r) = hazard.effects.risk_increase {
                hazard_risk += r as f32;
            }
            if hazard.effects.forced_choice == Some(true) {
                hazard_time += 5.0;
                hazard_risk += 1.0;
            }
        }
        fuel += hazard_fuel * skill_mods.hazard_mult;
        time += hazard_time * skill_mods.hazard_mult;
        risk = (risk + hazard_risk * skill_mods.hazard_mult).min(max_risk);

        // Route mastery bonuses
        if let Some(&mastery) = route_mastery.get(&route) {
            let fuel_reduction = (mastery / 3).min(4u32) as f32;
            let time_reduction = (mastery / 2).min(6u32) as f32;
            let risk_reduction = (mastery / 10).min(1u32) as f32;

            fuel = (fuel - fuel_reduction).max(constants.route_variations.minimum_fuel_cost as f32);
            time = (time - time_reduction).max(constants.route_variations.minimum_time_cost as f32);
            risk = (risk - risk_reduction).max(0.0);
        }

        // Fuel-efficiency skills (Hybrid Injection) scale the whole fuel cost.
        fuel = (fuel * skill_mods.fuel_cost_mult)
            .max(constants.route_variations.minimum_fuel_cost as f32);

        RouteCosts {
            fuel: fuel.round() as u32,
            time: time.round() as u32,
            risk: risk.round().clamp(0.0, max_risk) as u32,
        }
    }

    fn apply_location_modifiers(costs: &mut RouteCosts, state: &GameState, data: &GameData) {
        let Some(passenger) = state.current_passenger.as_ref() else {
            return;
        };
        let pickup = data.get_location(&passenger.pickup);
        let destination = data.get_location(&passenger.destination);
        let distance = pickup
            .map(|location| location.distance_multiplier)
            .unwrap_or(1.0)
            * destination
                .map(|location| location.distance_multiplier)
                .unwrap_or(1.0);
        let fuel = pickup
            .map(|location| location.fuel_multiplier)
            .unwrap_or(1.0)
            * destination
                .map(|location| location.fuel_multiplier)
                .unwrap_or(1.0);
        costs.time = (costs.time as f32 * distance).round() as u32;
        costs.fuel = (costs.fuel as f32 * fuel).round() as u32;
        costs.risk = (costs.risk as f32
            + destination
                .map(|location| location.destination_risk)
                .unwrap_or(0.0))
        .round() as u32;
    }

    fn apply_headlight_modifier(costs: &mut RouteCosts, state: &GameState) {
        if state.time_of_day.ambient_light >= 30 || state.headlights_on {
            return;
        }
        // Driving dark saves the electrical load but makes the road itself
        // more dangerous. The base calculator still owns the ordinary
        // nighttime headlight cost; this inverse keeps quotes and transit
        // consistent when the player deliberately turns the beam off.
        costs.fuel = (costs.fuel as f32 / 1.1).round() as u32;
        costs.risk = costs.risk.saturating_add(1);
    }

    /// Generate 3 risk tags for a route context.
    ///
    /// Ordered by a caller-supplied seed rather than any random stream: the
    /// one caller is the driving screen, which rebuilds these every frame —
    /// a draw here would either flicker the tags or, worse, bleed frame-rate
    /// into the gameplay stream.
    pub fn generate_risk_tags(
        route: RouteType,
        weather: Option<&WeatherCondition>,
        time_of_day: Option<&TimeOfDay>,
        _passenger: Option<&Passenger>,
        seed: u64,
    ) -> Vec<RiskTag> {
        let mut potential_risks = Vec::new();

        // Base risks by route type
        match route {
            RouteType::Normal => {
                potential_risks.push(RiskTag::HighTraffic);
                potential_risks.push(RiskTag::RoadConstruction);
                potential_risks.push(RiskTag::PolicePatrol);
            }
            RouteType::Shortcut => {
                potential_risks.push(RiskTag::Potholes);
                potential_risks.push(RiskTag::GangActivity);
                potential_risks.push(RiskTag::StrangeNoises);
            }
            RouteType::Scenic => {
                potential_risks.push(RiskTag::SlipperyRoads);
                potential_risks.push(RiskTag::SpiritualDisturbance);
                potential_risks.push(RiskTag::DenseFog);
            }
            RouteType::Police => {
                potential_risks.push(RiskTag::HighTraffic);
                potential_risks.push(RiskTag::SpiritualDisturbance); // Escort duty?
                potential_risks.push(RiskTag::PolicePatrol);
            }
        }

        // Weather risks
        if let Some(w) = weather {
            match w.weather_type {
                WeatherType::Rain | WeatherType::Thunderstorm => {
                    potential_risks.push(RiskTag::SlipperyRoads);
                    potential_risks.push(RiskTag::FlashFloods);
                }
                WeatherType::Fog => potential_risks.push(RiskTag::DenseFog),
                _ => {}
            }
        }

        // Time risks
        if let Some(t) = time_of_day {
            if matches!(t.phase, TimePhase::Night | TimePhase::Latenight) {
                potential_risks.push(RiskTag::StrangeNoises);
                potential_risks.push(RiskTag::SpiritualDisturbance);
            }
        }

        // Fill with generic risks if not enough
        let generics = [
            RiskTag::HighTraffic,
            RiskTag::RoadConstruction,
            RiskTag::Potholes,
        ];

        for r in generics.iter() {
            if !potential_risks.contains(r) {
                potential_risks.push(*r);
            }
        }

        potential_risks.sort_by_key(|risk| {
            let mut hasher = DefaultHasher::new();
            seed.hash(&mut hasher);
            risk.hash(&mut hasher);
            hasher.finish()
        });

        // Ensure unique
        let mut selected = Vec::new();
        for r in potential_risks {
            if !selected.contains(&r) && selected.len() < 3 {
                selected.push(r);
            }
        }

        // Fallback if still < 3 (shouldn't happen with generics adding)
        while selected.len() < 3 {
            selected.push(RiskTag::HighTraffic); // Safe fallback
        }

        selected
    }

    fn weather_effect_applies_to_route(effect: &WeatherEffect, route: RouteType) -> bool {
        let Some(applies_to) = effect.applies_to.as_deref() else {
            return true;
        };

        applies_to.eq_ignore_ascii_case(route.label()) || applies_to.eq_ignore_ascii_case("all")
    }
}
