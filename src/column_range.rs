use anyhow::bail;

/// Represents a range of columns (inclusive) in a given sheet in a Google Sheet.
#[derive(Debug)]
pub struct ColumnRange {
    pub(crate) sheet_name: String,

    // These are String and not char because columns after "Z" have multiple letters, e.g. "AA", "AB", "AC"
    pub(crate) start_col: String,
    pub(crate) end_col: String,
}

impl ColumnRange {
    /// Creates a new range for a specific sheet and column span.
    ///
    /// # Arguments
    /// * `sheet_name` - The name of the tab in the spreadsheet (e.g., "Sheet1").
    /// * `start_col` - The starting column letter (e.g., "A").
    /// * `end_col` - The ending column letter (e.g., "Z").
    pub fn new(sheet_name: &str, start_col: &str, end_col: &str) -> Self {
        Self {
            sheet_name: sheet_name.to_string(),
            start_col: start_col.to_string(),
            end_col: end_col.to_string(),
        }
    }

    /// Internal helper to format and URL-encode the range for the API.
    pub fn to_api_string(&self) -> String {
        let raw_range = format!("{}!{}:{}", self.sheet_name, self.start_col, self.end_col);
        // Use urlencoding to handle spaces and special characters safely
        urlencoding::encode(&raw_range).into_owned()
    }
}

impl TryFrom<&Vec<String>> for ColumnRange {
    type Error = anyhow::Error;

    /// Primarily used to convert command-line arguments into a ColumnRange.
    ///
    /// # Arguments
    /// * `parameters` - A vector of strings representing the sheet name, start column, and end column.
    fn try_from(parameters: &Vec<String>) -> Result<Self, Self::Error> {
        if parameters.len() != 3 {
            bail!("Expecting 3 parameters for column range, found {}", parameters.len());
        }
        Ok(Self::new(&parameters[0], &parameters[1], &parameters[2]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_range_new() {
        let range = ColumnRange::new("Sheet1", "A", "Z");
        assert_eq!(range.sheet_name, "Sheet1");
        assert_eq!(range.start_col, "A");
        assert_eq!(range.end_col, "Z");
    }

    #[test]
    fn test_to_api_string_basic() {
        let range = ColumnRange::new("Sheet1", "A", "C");
        // "Sheet1!A:C" -> "Sheet1%21A%3AC"
        assert_eq!(range.to_api_string(), "Sheet1%21A%3AC");
    }

    #[test]
    fn test_to_api_string_with_spaces() {
        let range = ColumnRange::new("My Sheet", "B", "AA");
        // "My Sheet!B:AA" -> "My%20Sheet%21B%3AAA"
        assert_eq!(range.to_api_string(), "My%20Sheet%21B%3AAA");
    }

    #[test]
    fn test_try_from_vec_success() {
        let params = vec![
            "Data".to_string(),
            "A".to_string(),
            "F".to_string(),
        ];
        let range = ColumnRange::try_from(&params).expect("Should succeed with 3 params");
        assert_eq!(range.sheet_name, "Data");
        assert_eq!(range.start_col, "A");
        assert_eq!(range.end_col, "F");
    }

    #[test]
    fn test_try_from_vec_wrong_length() {
        let params = vec!["Sheet1".to_string(), "A".to_string()];
        let result = ColumnRange::try_from(&params);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Expecting 3 parameters for column range, found 2"
        );
    }
}
