//! Local weather owns the shared inputs for lighting, particles and materials.
use crate::{
    performance::Performance,
    session::{Graphics, Player, Session},
};
use bevy::{audio::Volume, prelude::*};
use std::sync::Arc;
use worldcraft_core::{generation::unit, geology::gradient, Biome};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Clear,
    Rain,
    Thunderstorm,
    Snow,
    Sandstorm,
    Hurricane,
}

impl Kind {
    pub const ALL: [Self; 6] = [
        Self::Clear,
        Self::Rain,
        Self::Thunderstorm,
        Self::Snow,
        Self::Sandstorm,
        Self::Hurricane,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::Rain => "Rain",
            Self::Thunderstorm => "Thunderstorm",
            Self::Snow => "Snow",
            Self::Sandstorm => "Sandstorm",
            Self::Hurricane => "Hurricane",
        }
    }

    fn local(biome: Biome, temperature: f64, humidity: f64, coastal: bool, front: f64) -> Self {
        let arid = matches!(biome, Biome::Desert | Biome::Savanna) && humidity < 0.40;
        if coastal && temperature > 0.71 && humidity > 0.67 && front > 0.77 {
            Self::Hurricane
        } else if arid && front > 0.40 {
            Self::Sandstorm
        } else if (temperature < 0.21 || biome == Biome::Snow) && front > 0.18 {
            Self::Snow
        } else if !arid && humidity > 0.50 && temperature > 0.42 && front > 0.51 {
            Self::Thunderstorm
        } else if !arid && humidity > 0.34 && temperature >= 0.21 && front > 0.09 {
            Self::Rain
        } else {
            Self::Clear
        }
    }

    // Rain, snow, airborne dust, and wind strength.
    fn conditions(self) -> Vec4 {
        match self {
            Self::Clear => Vec4::ZERO,
            Self::Rain => Vec4::new(0.52, 0.0, 0.0, 0.23),
            Self::Thunderstorm => Vec4::new(0.86, 0.0, 0.0, 0.59),
            Self::Snow => Vec4::new(0.0, 0.66, 0.0, 0.24),
            Self::Sandstorm => Vec4::new(0.0, 0.0, 0.88, 0.81),
            Self::Hurricane => Vec4::new(1.0, 0.0, 0.0, 1.0),
        }
    }
}

#[derive(Resource, Default)]
pub struct Weather {
    pub kind: Kind,
    pub air: Vec4,
    pub wet: f32,
    pub snow: f32,
    pub dust: f32,
    pub flash: f32,
    pub strike: u64,
    pub strike_heading: f32,
    pub elapsed: f64,
    pub wind: Vec2,
    pub drift: Vec2,
    epoch: u64,
    next_sample: f64,
    next_strike: f64,
    last_strike: f64,
    thunder_at: f64,
}

impl Weather {
    pub fn overcast(&self) -> f32 {
        (self.air.x * 0.85 + self.air.y * 0.75 + self.air.z * 0.18).min(1.0)
    }

    pub fn visible_storm(&self) -> bool {
        self.overcast() > 0.01 || self.air.z > 0.01 || self.flash > 0.01
    }
}

pub fn update(
    time: Res<Time>,
    session: Res<Session>,
    player: Res<Player>,
    graphics: Res<Graphics>,
    limits: Res<Performance>,
    mut weather: ResMut<Weather>,
) {
    if weather.epoch != session.epoch {
        *weather = Weather {
            epoch: session.epoch,
            ..default()
        };
    }
    let Some(generator) = session.generator.as_ref() else {
        return;
    };
    if limits.suspended {
        return;
    }
    let dt = time.delta_secs();
    weather.elapsed += f64::from(dt);
    let elapsed = weather.elapsed;
    let seed = generator.settings().seed;
    if let Some(kind) = graphics.weather {
        weather.kind = kind;
        weather.next_sample = elapsed;
    } else if elapsed >= weather.next_sample {
        weather.next_sample = elapsed + 1.0;
        let x = player.position.x;
        let z = player.position.z;
        let sample = generator.preview_sample(x, z);
        let front = gradient(seed ^ 0x776541, x / 2500.0 + elapsed / 190.0, z / 2500.0)
            + 0.38 * gradient(seed ^ 0x776542, x / 900.0, z / 900.0 - elapsed / 110.0)
            + 0.21;
        let coastal = sample.height < generator.settings().sea_level + 52.0;
        weather.kind = Kind::local(
            sample.biome,
            sample.temperature,
            sample.moisture,
            coastal,
            front,
        );
    }

    let response = 1.0 - (-dt * 0.85).exp();
    weather.air = weather.air.lerp(weather.kind.conditions(), response);
    let target_wet = (weather.air.x * 1.10 + weather.air.y * 0.21).min(1.0);
    let wet_rate = if target_wet > weather.wet {
        0.24
    } else {
        0.032
    };
    weather.wet += (target_wet - weather.wet) * (1.0 - (-dt * wet_rate).exp());
    let target_snow = (weather.air.y * 1.48).min(1.0);
    let snow_rate = if target_snow > weather.snow {
        0.13
    } else {
        0.024
    };
    weather.snow += (target_snow - weather.snow) * (1.0 - (-dt * snow_rate).exp());
    let dust_rate = if weather.air.z > weather.dust {
        0.16
    } else {
        0.025 + weather.air.x * 0.15
    };
    weather.dust += (weather.air.z - weather.dust) * (1.0 - (-dt * dust_rate).exp());
    let turn = if weather.kind == Kind::Hurricane {
        (elapsed * 0.18 + player.position.x / 710.0 + player.position.z / 940.0).sin() as f32
            * weather.air.w
            * 0.8
    } else {
        0.0
    };
    let heading = std::f32::consts::TAU * unit(seed ^ 0x77696e64, 0, 0) as f32 + turn;
    let gust = 1.0 + (elapsed * 0.63).sin() as f32 * weather.air.w * 0.32;
    let speed = (2.0 * graphics.breeze + 23.0 * weather.air.w) * gust;
    weather.wind = Vec2::from_angle(heading) * speed;
    // Wrap at a multiple of the cloud noise period. Rain and dust use the same drift.
    weather.drift = (weather.drift + weather.wind * dt).rem_euclid(Vec2::splat(131072.0));

    if matches!(weather.kind, Kind::Thunderstorm | Kind::Hurricane) && weather.air.x > 0.35 {
        if weather.next_strike == 0.0 {
            weather.next_strike = elapsed + 1.6 + unit(seed ^ 0x57, weather.strike as i64, 0) * 2.0;
        }
        if elapsed >= weather.next_strike {
            weather.strike += 1;
            weather.last_strike = elapsed;
            weather.next_strike =
                elapsed + 6.0 + unit(seed ^ 0x57, weather.strike as i64, 0) * 10.0;
            weather.thunder_at = elapsed + 0.65 + unit(seed ^ 0x58, weather.strike as i64, 0) * 1.8;
            weather.strike_heading =
                std::f32::consts::TAU * unit(seed ^ 0x59, weather.strike as i64, 0) as f32;
        }
    } else {
        weather.next_strike = 0.0;
    }
    let since = (elapsed - weather.last_strike) as f32;
    weather.flash = if weather.strike > 0 && since < 0.65 {
        (-since * 23.0).exp() + 0.44 * (-(since - 0.14).abs() * 42.0).exp()
    } else {
        0.0
    };
}

#[derive(Resource)]
pub struct Thunder(Handle<AudioSource>);

#[derive(Component)]
pub struct ThunderVoice {
    epoch: u64,
    end: f64,
}

pub fn setup_audio(mut commands: Commands, mut sounds: ResMut<Assets<AudioSource>>) {
    commands.insert_resource(Thunder(sounds.add(AudioSource {
        bytes: Arc::from(thunder_wave()),
    })));
}

pub fn sound(
    mut commands: Commands,
    mut weather: ResMut<Weather>,
    thunder: Res<Thunder>,
    voices: Query<(Entity, &ThunderVoice)>,
) {
    for (entity, voice) in &voices {
        if voice.epoch != weather.epoch || weather.elapsed >= voice.end {
            commands.entity(entity).try_despawn();
        }
    }
    if weather.thunder_at > 0.0 && weather.elapsed >= weather.thunder_at {
        weather.thunder_at = 0.0;
        commands.spawn((
            Name::new("Distant thunder"),
            ThunderVoice {
                epoch: weather.epoch,
                end: weather.elapsed + 3.5,
            },
            AudioPlayer::new(thunder.0.clone()),
            PlaybackSettings::ONCE.with_volume(Volume::Linear(0.48)),
        ));
    }
}

fn thunder_wave() -> Vec<u8> {
    const RATE: u32 = 24_000;
    const SAMPLES: u32 = RATE * 3;
    let bytes = SAMPLES * 2;
    let mut wav = Vec::with_capacity((bytes + 44) as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(bytes + 36).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1_u16.to_le_bytes()); // Mono
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes()); // Bytes per frame
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&bytes.to_le_bytes());
    let mut noise = 0x8126ac7_u32;
    let mut body = 0.0;
    let mut roll = 0.0;
    for i in 0..SAMPLES {
        noise ^= noise << 13;
        noise ^= noise >> 17;
        noise ^= noise << 5;
        let white = noise as f32 * (2.0 / u32::MAX as f32) - 1.0;
        body += 0.11 * (white - body);
        roll += 0.022 * (body - roll);
        let time = i as f32 / RATE as f32;
        let attack = (time * 22.0).min(1.0);
        let tail = (1.0 - time / 3.0).powi(2);
        let crack = (-18.0 * time).exp() * body * 0.8;
        let echo = 0.67 + 0.33 * (time * 15.0 + time * time * 2.0).cos();
        let sample = attack * tail * (crack + 4.7 * roll * echo + 0.42 * body);
        wav.extend_from_slice(&((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::audio::Source;
    use worldcraft_core::{GeneratorSettings, Preset};

    #[test]
    fn thunder_clip_decodes() {
        let clip = AudioSource {
            bytes: Arc::from(thunder_wave()),
        };
        let mut decoder = clip.decoder();
        assert_eq!(decoder.sample_rate().get(), 24_000);
        assert_eq!(decoder.channels().get(), 1);
        assert!(decoder
            .by_ref()
            .take(2400)
            .any(|sample| sample.abs() > 0.05));
    }

    #[test]
    fn storm_wets_surfaces_before_the_thunder_roll_and_recovers_afterward() {
        let mut app = App::new();
        let mut session = Session::default();
        let mut player = Player::default();
        session.create(
            "Weather test".into(),
            GeneratorSettings::for_preset(Preset::Temperate, 42),
            &mut player,
        );
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(125),
            ))
            .insert_resource(session)
            .insert_resource(player)
            .insert_resource(Performance::load(true))
            .insert_resource(Graphics {
                weather: Some(Kind::Thunderstorm),
                ..default()
            })
            .init_resource::<Weather>()
            .add_systems(Update, update);
        for _ in 0..64 {
            app.update();
        }
        let storm = app.world().resource::<Weather>();
        assert!(storm.wet > 0.5 && storm.strike == 1);
        assert!(storm.thunder_at > storm.last_strike + 0.5);
        let wet = storm.wet;
        app.world_mut().resource_mut::<Graphics>().weather = Some(Kind::Clear);
        for _ in 0..40 {
            app.update();
        }
        let clear = app.world().resource::<Weather>();
        assert!(clear.air.x < 0.02 && clear.wet > 0.25 && clear.wet < wet);
    }

    #[test]
    fn fronts_follow_local_climate() {
        assert_eq!(
            Kind::local(Biome::Desert, 0.90, 0.13, false, 0.64),
            Kind::Sandstorm
        );
        assert_eq!(
            Kind::local(Biome::Snow, 0.08, 0.45, false, 0.60),
            Kind::Snow
        );
        assert_eq!(
            Kind::local(Biome::Forest, 0.54, 0.65, false, 0.64),
            Kind::Thunderstorm
        );
        assert_eq!(
            Kind::local(Biome::Rainforest, 0.85, 0.80, true, 0.85),
            Kind::Hurricane
        );
        assert_eq!(
            Kind::local(Biome::Rainforest, 0.85, 0.80, false, 0.85),
            Kind::Thunderstorm
        );
        assert_eq!(
            Kind::local(Biome::Grassland, 0.52, 0.45, false, 0.27),
            Kind::Rain
        );
        assert_eq!(
            Kind::local(Biome::Grassland, 0.52, 0.45, false, -0.1),
            Kind::Clear
        );
    }
}
