use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file name
    #[arg(long, default_value = "src/generate/liquid_prompt_config.yaml")]
    liquid_in_file: String,
    /// Template templates directory
    #[arg(long, default_value = "src/generate/template_templates/liquid")]
    liquid_template_templates_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TemplateConfig {
    RawTemplate(String),
    Config {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        variables: HashMap<String, TemplateConfig>,
    },
}

impl TemplateConfig {
    fn get_template_string(&self, base_dir: &String) -> Result<String> {
        match self {
            TemplateConfig::RawTemplate(s) => Ok(s.clone()),
            TemplateConfig::Config {
                path,
                variables,
            } => {
                let path_str = path.as_ref().ok_or_else(|| anyhow::anyhow!("Template path is required but not provided"))?;
                let full_path = format!("{}/{}", base_dir, path_str);
                let content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {:?}", full_path))?;

                // Process variables
                if !variables.is_empty() {
                    let mut variable_strings = Vec::new();
                    let mut keys: Vec<_> = variables.keys().collect();
                    keys.sort(); // Sort for consistent output

                    for key in keys {
                        let var_content = variables[key].get_template_string(base_dir)?;
                        let var_content_trimmed = var_content.trim_end_matches('\n');
                        variable_strings.push(format!(
                            "{{% capture {} %}}{}{{% endcapture %}}",
                            key, var_content_trimmed
                        ));
                    }

                    let variables_string = variable_strings.join("");
                    Ok(format!("{}{}", variables_string, content))
                } else {
                    Ok(content)
                }
            }
        }
    }
}

fn parse_and_render(args: &Args) -> Result<()> {
    let liquid_in_file = &args.liquid_in_file;
    let liquid_template_templates_dir = &args.liquid_template_templates_dir;

    // Read the input config file
    let input_content = fs::read_to_string(&liquid_in_file)
        .with_context(|| format!("Failed to read input file: {:?}", liquid_in_file))?;

    let config: HashMap<String, TemplateConfig> = serde_yaml::from_str(&input_content)
        .with_context(|| format!("Failed to parse YAML from: {:?}", liquid_in_file))?;

    for (key, template_config) in &config {
        let template_string = template_config.get_template_string(liquid_template_templates_dir)
            .with_context(|| format!("Failed to process template for key: {}", key))?;
        let out_file_path = format!("gen/templates/liquid/{}.tmpl", key);

        fs::write(&out_file_path, template_string)
            .with_context(|| format!("Failed to write output file: {:?}", out_file_path))?;
    }

    println!("Successfully generated {} liquid templates from {}", config.len(), args.liquid_in_file);
    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(e) = parse_and_render(&args) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

