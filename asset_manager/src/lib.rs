mod asset_type;

use crate::asset_type::AssetType;
use bevy::prelude::*;
use std::any::Any;
use std::collections::HashMap;

pub struct AssetManagerPlugin;

pub mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

pub(crate) trait ManagedAsset {
    fn get_asset_type(&self) -> AssetType;

    fn get_path(&self) -> &str;
}

impl Plugin for AssetManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_assets);
    }
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut fonts = HashMap::new();
    let mut images = HashMap::new();
    let mut audio_sources = HashMap::new();

    insert_into_assets(&mut fonts, &asset_server, &assets::FONTS);
    insert_into_assets(&mut images, &asset_server, &assets::IMAGES);
    insert_into_assets(&mut audio_sources, &asset_server, &assets::AUDIO_SOURCES);

    commands.insert_resource(AssetManager {
        fonts,
        images,
        audio_sources,
    });
}

fn insert_into_assets<'a, A: Asset>(
    assets: &mut HashMap<&'a str, Handle<A>>,
    asset_server: &Res<AssetServer>,
    managed_assets: &[&'a dyn ManagedAsset],
) {
    for asset in managed_assets {
        assets.insert(asset.get_path(), asset_server.load(asset.get_path()));
    }
}

#[derive(Resource)]
pub struct AssetManager {
    fonts: HashMap<&'static str, Handle<Font>>,
    images: HashMap<&'static str, Handle<Image>>,
    audio_sources: HashMap<&'static str, Handle<AudioSource>>,
}

impl AssetManager {
    #[allow(private_bounds)]
    pub fn get<A: Asset>(&self, managed_asset: impl ManagedAsset) -> Handle<A> {
        let handle = match managed_asset.get_asset_type() {
            AssetType::Font => &self.fonts[managed_asset.get_path()] as &dyn Any,
            AssetType::Image => &self.images[managed_asset.get_path()] as &dyn Any,
            AssetType::AudioSource => &self.audio_sources[managed_asset.get_path()] as &dyn Any,
        };
        handle.downcast_ref::<Handle<A>>().expect("handle").clone()
    }
}
