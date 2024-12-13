use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Meta {
    pub id: Id,
    pub name: String,
    pub color: Color,
    pub version: String,
    pub entrypoint: PathBuf,
    // TODO: do this properly (by nesting types)
    pub instance: u8,
}

#[derive(Debug)]
pub struct LoadMetaError(pub String);

impl Meta {
    const DEFAULT_COLOR: Color = Color {
        red: 100,
        green: 100,
        blue: 100,
    };

    pub fn display_name(&self) -> String {
        let instance_counter = if self.instance == 1 {
            String::new()
        } else {
            format!(" ({})", self.instance)
        };
        format!("{}_{}{}", self.name, self.version, instance_counter)
    }

    // FIXME: add proper error handling and refactor
    fn from_toml_str(toml: &str) -> Result<Self, LoadMetaError> {
        let table = toml
            .parse::<toml::Table>()
            .map_err(|_| LoadMetaError("Could not parse TOML table".to_string()))?;
        let name = table["name"].as_str().unwrap().to_string();
        let raw_id = table["id"].as_str().unwrap();
        let id = uuid::Uuid::parse_str(raw_id)
            .map_err(|_| LoadMetaError(format!("expected valid UUID, got {raw_id}")))?
            .into();
        let version = table
            .get("version")
            .map_or("1.0", |v| v.as_str().unwrap_or("1.0"))
            .to_string();
        let entrypoint = table
            .get("entrypoint")
            .ok_or(LoadMetaError(format!("'entrypoint' missing")))?
            .as_str()
            .ok_or(LoadMetaError(format!("'entrypoint' is not a string")))?
            .to_string()
            .into();
        let color = table.get("color").map_or(Self::DEFAULT_COLOR, |c| {
            c.as_table()
                .map(|color_table| Color {
                    red: color_table["red"].as_integer().unwrap() as u8,
                    green: color_table["green"].as_integer().unwrap() as u8,
                    blue: color_table["blue"].as_integer().unwrap() as u8,
                })
                .unwrap_or(Self::DEFAULT_COLOR)
        });
        Ok(Self {
            name,
            id,
            version,
            entrypoint,
            color,
            instance: 1,
        })
    }

    pub fn from_toml_file(path: &Path) -> Result<Self, LoadMetaError> {
        let contents = std::fs::read_to_string(path).map_err(|e| LoadMetaError(e.to_string()))?;
        Self::from_toml_str(&contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod meta {
        use super::*;

        #[test]
        fn can_be_loaded_from_toml_string() {
            let toml_str = "
name = \"Kai\"
id = \"00000000-0000-0000-0000-000000000000\"
version = \"1.09c\"
entrypoint = \"main.lua\"
[color]
red = 243
green = 0
blue = 13
";
            let meta = Meta::from_toml_str(toml_str).unwrap();
            assert_eq!(meta.name, "Kai");
            assert_eq!(
                meta.id,
                Id(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
            );
            assert_eq!(meta.version, "1.09c");
            assert_eq!(meta.entrypoint.to_str().unwrap(), "main.lua");
            assert_eq!(
                meta.color,
                Color {
                    red: 243,
                    green: 0,
                    blue: 13
                }
            );
        }

        #[test]
        fn version_has_default_value() {
            // TODO: add `Version` implementing Default
            let toml_str = "
name = \"Kai\"
id = \"00000000-0000-0000-0000-000000000000\"
entrypoint = \"kai.lua\"
";
            let meta = Meta::from_toml_str(toml_str).unwrap();
            assert_eq!(meta.version, "1.0");
        }

        #[test]
        fn color_has_default_value() {
            // TODO: add `CharacterColor` implementing Default
            let toml_str = "
name = \"Nya\"
id = \"00000000-0000-0000-0000-000000000000\"
entrypoint = \"nya.wasm\"
";
            let meta = Meta::from_toml_str(toml_str).unwrap();
            assert_eq!(meta.color, Meta::DEFAULT_COLOR);
        }
    }
}
