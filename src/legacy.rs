use ::types::*;

impl Options {
    pub fn legacy_defaults(datamodel: Datamodel) -> Options {
        Options {
            datamodel: datamodel,
            case_to_file_paths: Vec::new(),
            file_labels: Vec::new(),
            possible_associated_entites: Vec::new(),
            index_file_extensions: Vec::new(),
            index_type: IndexType::Legacy,
        }
    }
}
