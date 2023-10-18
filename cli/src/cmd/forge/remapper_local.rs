use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub struct LocalRemapper {
    temp_directory: String,
    changed_filenames: HashMap<PathBuf, (PathBuf, String)>,
    project_paths: HashMap<String, PathBuf>, // Store various project paths
    remappings: HashMap<String, String>,     // Store remapping rules
    contract_name_count: HashMap<String, u32>, // Store counts for duplicate contract names
    contract_content_map: HashMap<String, String>, // Store contract contents
    contract_paths_map: HashMap<PathBuf, PathBuf>, // Map original paths to new paths after remapping
}

impl LocalRemapper {
    pub fn new(
        temp_directory: &str,
        project_paths: HashMap<String, PathBuf>,
        remappings: HashMap<String, String>,
    ) -> Self {
        Self {
            temp_directory: temp_directory.to_string(),
            changed_filenames: HashMap::new(),
            project_paths, // Initialized with the passed argument
            remappings,    // Initialized with the passed argument
            contract_name_count: HashMap::new(),
            contract_content_map: HashMap::new(),
            contract_paths_map: HashMap::new(),
        }
    }

    pub fn apply_local_remapping(
        &mut self,
        standard_json: StandardJsonCompilerInput,
        main_contract_path: &PathBuf,
    ) -> Result<(), Error> {
        // Iterate through the contracts in standard_json and apply the local remapping logic
        for (path, source) in &standard_json.sources {
            // Determine if the current contract is the main contract
            let is_main_contract = path == main_contract_path;

            // Check for duplicate names and get a new contract name if necessary
            let new_contract_name = self.check_and_rename_contract(
                path,
                &source.content,  // Passing the source content
                is_main_contract, // Passing whether it's the main contract
            )?;

            // Get new path based on the new contract name
            let new_path = self.get_new_path(path, &new_contract_name)?;

            // Get the directory of the current contract
            let current_contract_dir = path
                .parent()
                .ok_or_else(|| Error::msg("Failed to get the directory of the current contract"))?;

            // Update the content of the source, modifying paths in the import statements
            let new_content =
                self.update_content(source.content.to_string(), &current_contract_dir)?;

            // Write the updated contract content to the new path
            self.write_contract(&new_path, &new_content)?;
        }
        Ok(())
    }

    fn check_and_rename_contract(
        &mut self,
        path: &PathBuf,
        source_content: &str,
        is_main_contract: bool,
    ) -> Result<(String, Option<(PathBuf, String)>), Error> {
        // Extract the contract name from the path (file stem)
        let contract_name = path
            .file_stem()
            .ok_or_else(|| Error::msg("Failed to extract contract name from path"))?
            .to_str()
            .ok_or_else(|| Error::msg("Failed to convert contract name to string"))?
            .to_string();

        // Check if the contract name already exists in the map and handle duplication
        if let Some(existing_content) = self.contract_content_map.get(&contract_name) {
            if existing_content != source_content {
                // Same name, different content: Handle duplicate contract names
                let counter = self.contract_name_count.entry(contract_name.clone()).or_insert(0);
                *counter += 1;

                let new_contract_name = format!("{}_{}", contract_name, counter);
                let new_filename = format!("{}.sol", new_contract_name);
                let new_path = PathBuf::from(format!("{}/{}", self.temp_directory, new_filename));

                return Ok((new_contract_name, Some((new_path, new_filename))));
            }
        } else {
            // New contract name: Store the content
            self.contract_content_map.insert(contract_name.clone(), source_content.to_string());
        }

        // If the contract name is not duplicated or is the main contract, no renaming occurs
        Ok((contract_name, None))
    }

    fn get_new_path(
        &self,
        original_path: &PathBuf,
        new_contract_name: &str,
    ) -> Result<PathBuf, Error> {
        // Get the directory of the original file
        let original_dir = original_path
            .parent()
            .ok_or_else(|| Error::msg("Failed to get the directory of the original file"))?;

        // Combine the original directory with the temp_directory to create a new directory path
        let new_dir = original_dir.join(&self.temp_directory);

        // Ensure the new directory exists
        if !new_dir.exists() {
            fs::create_dir_all(&new_dir)
                .map_err(|e| Error::msg(format!("Failed to create directory: {}", e)))?;
        }

        // Create the new file path by combining the new directory with the new contract name
        let new_file_path = new_dir.join(format!("{}.sol", new_contract_name));

        Ok(new_file_path)
    }

    fn update_content(
        &self,
        content: String,
        current_contract_dir: &PathBuf,
    ) -> Result<String, Error> {
        let regex =
            Regex::new(r#"import\s+(?P<items>\{.*?\}\s+from\s+)?["'](?P<path>[^"']+)["'];"#)
                .map_err(|_| Error::msg("Failed to compile regex"))?;

        // Modifying the import paths in the content
        let modified_content = regex
            .replace_all(&content, |caps: &regex::Captures| {
                let import_path_str = caps.name("path").unwrap().as_str();
                let items = caps.name("items").map_or("", |m| m.as_str());

                // Resolve the import path to its canonical path
                let canonical_import_path = current_contract_dir
                    .join(import_path_str)
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from(import_path_str));

                // Check if the canonical import path is in the changed_filenames map
                if let Some((_, new_filename)) = self.changed_filenames.get(&canonical_import_path)
                {
                    // If it is, modify the import statement with the new filename
                    format!("import {}\"./{}\";", items, new_filename)
                } else {
                    // If it's not in the map, keep the import statement as it is
                    format!("import {}\"{}\";", items, import_path_str)
                }
            })
            .to_string();

        Ok(modified_content)
    }

    fn write_contract(&self, path: &PathBuf, content: &str) -> Result<(), Error> {
        // Logic to write the updated contract content to the new path
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}
