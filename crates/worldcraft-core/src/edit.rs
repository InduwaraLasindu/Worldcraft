use serde::{Deserialize, Serialize};
use crate::{config::MAX_WORLD_COORD, Biome};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrushKind {
    #[default]
    Raise,
    Lower,
    Flatten,
    Smooth,
    Terrace,
    Biome,
    Vegetation,
}
impl BrushKind {
    pub const ALL: [Self; 7] = [Self::Raise, Self::Lower, Self::Flatten, Self::Smooth,
        Self::Terrace, Self::Biome, Self::Vegetation];
    pub fn label(self) -> &'static str {
        match self {
            Self::Raise => "Raise", Self::Lower => "Lower", Self::Flatten => "Flatten",
            Self::Smooth => "Smooth", Self::Terrace => "Terrace",
            Self::Biome => "Paint biome", Self::Vegetation => "Paint vegetation",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Brush {
    pub kind: BrushKind,
    pub radius: f64,
    pub strength: f64,
    pub target_height: f64,
    pub biome: Biome,
    pub density: f64,
}
impl Default for Brush {
    fn default() -> Self {
        Self { kind: BrushKind::Raise, radius: 24.0, strength: 5.0,
            target_height: 0.0, biome: Biome::Forest, density: 1.0 }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Stroke {
    pub center: [f64; 2],
    pub brush: Brush,
}
impl Stroke {
    pub fn validate(&self) -> Result<(), String> {
        let b = &self.brush;
        if !self.center.iter().all(|v| v.is_finite() && v.abs() <= MAX_WORLD_COORD)
            || !b.radius.is_finite() || !(1.0..=512.0).contains(&b.radius)
            || !b.strength.is_finite() || !(0.0..=100.0).contains(&b.strength)
            || !b.target_height.is_finite() || b.target_height.abs() > 100_000.0
            || !b.density.is_finite() || !(0.0..=2.0).contains(&b.density) {
            return Err("Invalid terrain brush or out-of-range position".into());
        }
        Ok(())
    }
    pub fn weight(&self, x: f64, z: f64) -> f64 {
        let d = ((x - self.center[0]).powi(2) + (z - self.center[1]).powi(2)).sqrt();
        let t = (1.0 - d / self.brush.radius).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
    pub fn height(&self, h: f64, weight: f64) -> f64 {
        let b = &self.brush;
        match b.kind {
            BrushKind::Raise => h + b.strength * weight,
            BrushKind::Lower => h - b.strength * weight,
            BrushKind::Flatten | BrushKind::Smooth =>
                h + (b.target_height - h) * weight * (b.strength / 10.0).clamp(0.0, 1.0),
            BrushKind::Terrace => {
                let step = b.strength.max(1.0);
                h + ((h / step).round() * step - h) * weight
            }
            _ => h,
        }
    }
}

/// A press/drag/release is one transaction, regardless of its dab count.
#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    pub transactions: Vec<Vec<Stroke>>,
    pub redo: Vec<Vec<Stroke>>,
    pub revision: u64,
}
impl EditHistory {
    pub fn from_transactions(transactions: Vec<Vec<Stroke>>) -> Self {
        Self { transactions, redo: Vec::new(), revision: 0 }
    }
    pub fn push(&mut self, strokes: Vec<Stroke>) -> Result<(), String> {
        if strokes.is_empty() { return Ok(()); }
        if self.transactions.iter().map(Vec::len).sum::<usize>() + strokes.len() > 100_000 {
            return Err("This world has reached the current 100,000 brush-dab limit".into());
        }
        for s in &strokes { s.validate()?; }
        self.transactions.push(strokes);
        self.redo.clear();
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn undo(&mut self) -> bool {
        if let Some(s) = self.transactions.pop() {
            self.redo.push(s); self.revision = self.revision.wrapping_add(1); true
        } else { false }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(s) = self.redo.pop() {
            self.transactions.push(s); self.revision = self.revision.wrapping_add(1); true
        } else { false }
    }
    pub fn strokes(&self) -> Vec<Stroke> {
        self.transactions.iter().flatten().cloned().collect()
    }
}
