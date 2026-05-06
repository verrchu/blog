pub mod rust_yaml;
pub mod yaml_edit;
pub mod yamlpatch;
pub mod yamp;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Lib {
    Yamp,
    RustYaml,
    YamlEdit,
    Yamlpatch,
}

impl Lib {
    pub fn list_assets(self, input: &str, assets: &[String]) -> anyhow::Result<String> {
        match self {
            Lib::Yamp => yamp::list_assets(input, assets),
            Lib::RustYaml => rust_yaml::list_assets(input, assets),
            Lib::YamlEdit => yaml_edit::list_assets(input, assets),
            Lib::Yamlpatch => yamlpatch::list_assets(input, assets),
        }
    }

    pub fn delist_assets(self, input: &str, assets: &[String]) -> anyhow::Result<String> {
        match self {
            Lib::Yamp => yamp::delist_assets(input, assets),
            Lib::RustYaml => rust_yaml::delist_assets(input, assets),
            Lib::YamlEdit => yaml_edit::delist_assets(input, assets),
            Lib::Yamlpatch => yamlpatch::delist_assets(input, assets),
        }
    }
}
