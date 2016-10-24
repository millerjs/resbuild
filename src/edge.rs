use ::types::*;
use crypto::md5::Md5;
use crypto::digest::Digest;
use std::str;

fn prefix(s: String, k: usize) -> String {
    let idx = s.char_indices().nth(k).map(|(idx, _)| idx).unwrap_or(s.len());
    s[0..idx].to_string()
}

impl Edge {
    pub fn new<S>(label: S, src_id: S, dst_id: S) -> Edge
        where S: Into<String>
    {
        Edge {
            label: label.into(),
            src_id: src_id.into(),
            dst_id: dst_id.into(),
        }
    }
}


impl EdgeType {
    /// Generate a name for the edge table.
    ///
    /// Because of the limit on table name length on PostgreSQL, we have
    /// to truncate some of the longer names.  To do this we concatenate
    /// the first 2 characters of each word in each of the input arguments
    /// up to 10 characters (per argument).  However, this strategy would
    /// very likely lead to collisions in naming.  Therefore, we take the
    /// first 8 characters of a hash of the full, un-truncated name
    /// *before* we truncate and prepend this to the truncation.  This
    /// gets us a name like ``edge_721d393f_LaLeSeqDaFrLaLeSeBu``.  This
    /// is rather an undesirable workaround. - jsm
    pub fn get_tablename(&self) -> String
    {

        let tablename = format!(
            "edge_{}{}{}",
            self.src_label.replace("_", ""),
            self.label.replace("_", ""),
            self.dst_label.replace("_", ""),
        );

        // If the name is too long, prepend it with the first 8 hex of it"s hash
        // truncate the each part of the name
        if tablename.len() > 40 {
            let mut digest = [0; 16];
            let mut hasher = Md5::new();
            hasher.input(tablename.as_bytes());
            hasher.result(&mut digest);

            let part1 = prefix(self.src_label.split("_")
                .map(|s| s[..2].to_string()).collect::<Vec<_>>().join(""), 10);
            let part2 = &*prefix(self.label.split("_")
                .map(|s| s[..2].to_string()).collect::<Vec<_>>().join(""), 10);
            let part3 = prefix(self.dst_label.split("_")
                .map(|s| s[..2].to_string()).collect::<Vec<_>>().join(""), 10);

            let hash = digest[..4].iter()
                .map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("");
            let suffix = format!("{}{}{}", part1, part2, part3);

            format!("edge_{}_{}", hash, suffix)

        } else {
            tablename
        }
    }
}
