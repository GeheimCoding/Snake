use crate::asset_type::AssetType;
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

mod asset_type {
    include!("src/asset_type.rs");
}

const ASSET_PATH: &str = "assets";

fn main() -> Result<()> {
    println!("cargo::rerun-if-changed=../{ASSET_PATH}");
    generate_enums()?;
    Ok(())
}

pub fn generate_enums() -> Result<()> {
    let mut asset_map = HashMap::new();
    let assets = traverse(format!("../{ASSET_PATH}"));
    for asset in assets.iter().filter(|asset| asset.extension().is_some()) {
        let (key, value) = to_entry(asset);
        if AssetType::from_str(&to_mod(&value)).is_ok() {
            let entry = asset_map
                .entry(String::from(&key[3..])) // ../
                .or_insert_with(Vec::new);
            entry.push(value);
        }
    }
    let mut folders = HashSet::new();
    for key in asset_map.keys() {
        let mut path = key.as_str();
        while let Some(index) = path.rfind(".") {
            path = &path[..index];
            folders.insert(path);
        }
    }

    let mut type_map = HashMap::new();
    let mut token_map = HashMap::new();
    for (key, value) in asset_map.iter() {
        let name = format_ident!("{}", to_enum(key));
        let values = value
            .iter()
            .map(|v| get_without_last_part(v))
            .collect::<Vec<_>>();
        let variants = values
            .iter()
            .map(|v| format_ident!("{}", to_enum(v)))
            .collect::<Vec<_>>();
        let asset_types = value
            .iter()
            .map(|v| AssetType::from_str(&to_mod(v)).expect("should have been filtered out"))
            .map(|a| format_ident!("{a:?}"))
            .collect::<Vec<_>>();
        let start = if key == ASSET_PATH {
            ASSET_PATH.len()
        } else {
            ASSET_PATH.len() + 1
        };
        let path = &key[start..].replace(".", "/");
        let paths = value
            .iter()
            .map(|v| format!("{path}{}{v}", if path.is_empty() { "" } else { "/" }))
            .collect::<Vec<_>>();

        let path = if path.is_empty() {
            to_enum(key)
        } else {
            let parts = path.split('/').collect::<Vec<_>>();
            format!(
                "{}{}{}",
                parts[..parts.len() - 1]
                    .iter()
                    .map(|p| to_mod(p))
                    .collect::<Vec<_>>()
                    .join("::"),
                if parts.len() > 1 { "::" } else { "" },
                to_enum(parts.last().expect("should have last part"))
            )
        };
        for (index, value) in value.iter().enumerate() {
            let ident = format!("&{path}::{}", to_enum(values[index]));
            let asset_type =
                AssetType::from_str(&to_mod(value)).expect("should have been filtered out");
            let entry = type_map.entry(asset_type).or_insert_with(Vec::new);
            entry.push(ident);
        }

        // TODO: cleanup
        // TODO: move to build script

        let tokens = quote! {
            pub enum #name {
                #( #variants, )*
            }

            impl std::str::FromStr for #name {
                type Err = ();
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        #( #values => Ok(Self::#variants), )*
                        _ => Err(()),
                    }
                }
            }

            impl crate::ManagedAsset for #name {
                fn get_asset_type(&self) -> crate::AssetType {
                    match self {
                        #( Self::#variants => crate::AssetType::#asset_types, )*
                    }
                }

                fn get_path(&self) -> &str {
                    match self {
                        #( Self::#variants => #paths, )*
                    }
                }
            }
        };
        token_map.insert(key.clone(), tokens.clone());
    }

    let mut keys = asset_map.keys().cloned().collect::<Vec<_>>();
    for &folder in &folders {
        if !keys.contains(&String::from(folder)) {
            keys.push(String::from(folder));
        }
        if !token_map.contains_key(&String::from(folder)) {
            token_map.insert(String::from(folder), quote! {});
        }
    }
    keys.sort_by(|a, b| count(b, '.').cmp(&count(a, '.')));

    let mut nested_map = HashMap::new();
    for key in &keys {
        if let Some(index) = key.rfind(".") {
            let entry = nested_map.entry(&key[..index]).or_insert_with(Vec::new);
            entry.push(key);
        }
    }

    for key in &keys {
        if let Some(initial_tokens) = token_map.get(key).cloned() {
            let tokens = if let Some(nested) = nested_map.get(key.as_str()) {
                let mut nested = nested.clone();
                nested.sort_by(|a, b| a.cmp(b));

                let added_tokens = nested
                    .iter()
                    .map(|n| token_map.get(*n).unwrap())
                    .collect::<Vec<_>>();
                quote! {
                    #( #added_tokens )*
                }
            } else {
                quote! {}
            };
            let mod_name = format_ident!("{}", to_mod(key));
            let tokens = if key.contains(".") && folders.contains(key.as_str()) {
                quote! {
                    pub mod #mod_name {
                        #tokens
                    }
                }
            } else {
                tokens
            };
            token_map.insert(
                key.clone(),
                quote! {
                    #tokens
                    #initial_tokens
                },
            );
        }
    }
    let combined = token_map.get(ASSET_PATH).unwrap_or(&quote! {}).clone();

    let fonts = collect_assets("FONTS", &mut type_map, AssetType::Font);
    let images = collect_assets("IMAGES", &mut type_map, AssetType::Image);
    let audio_sources = collect_assets("AUDIO_SOURCES", &mut type_map, AssetType::AudioSource);
    let combined = quote! {
        #combined
        #fonts
        #images
        #audio_sources
    };

    let file = syn::parse2(combined)?;
    let string = prettyplease::unparse(&file);

    fs::write(format!("{}/assets.rs", std::env::var("OUT_DIR")?), string)?;
    Ok(())
}

fn collect_assets(
    ident: &str,
    type_map: &mut HashMap<AssetType, Vec<String>>,
    asset_type: AssetType,
) -> TokenStream {
    let ident = format_ident!("{ident}");
    type_map
        .get_mut(&asset_type)
        .map(|assets| {
            assets.sort();
            let len = assets.len();
            let paths = assets
                .iter()
                .map(|f| f.parse::<TokenStream>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .expect("valid paths");
            quote! {
                pub(crate) const #ident: [&dyn crate::ManagedAsset; #len] = [#( #paths ),*];
            }
        })
        .unwrap_or(quote! {
                    pub(crate) const #ident: [&dyn crate::ManagedAsset; 0usize] = [];

        })
}

fn count(string: &str, ch: char) -> usize {
    string.chars().filter(|c| *c == ch).count()
}

fn get_last_part(string: &str) -> &str {
    &string[string.rfind(".").map(|i| i + 1).unwrap_or_default()..]
}

fn get_without_last_part(string: &str) -> &str {
    &string[..string.rfind(".").unwrap_or(string.len())]
}

fn to_mod(input: &str) -> String {
    get_last_part(input).to_case(Case::Snake)
}

fn to_enum(input: &str) -> String {
    get_last_part(input).to_case(Case::Pascal)
}

fn to_entry(path: &PathBuf) -> (String, String) {
    let mut dir = vec![];
    for part in path.iter().filter(|part| path.file_name() != Some(part)) {
        dir.push(String::from(part.to_str().unwrap()));
    }
    (
        dir.join("."),
        path.file_name().unwrap().to_str().unwrap().to_string(),
    )
}

fn traverse(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return vec![];
    };
    entries
        .flatten()
        .flat_map(|entry| {
            let Ok(metadata) = entry.metadata() else {
                return vec![];
            };
            if metadata.is_dir() {
                return traverse(entry.path());
            }
            if metadata.is_file() {
                return vec![entry.path()];
            }
            vec![]
        })
        .collect()
}
