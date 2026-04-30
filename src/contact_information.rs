use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct ContactInformation {
    #[serde(rename = "Precinct")]
    pub precinct: String,
    /*
    #[serde(rename = "STATUS")]
    pub status: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Preferred Email")]
    pub email_address: String,
    #[serde(rename = "phone")]
    pub phone_number: String,
    #[serde(rename = "Street Address")]
    pub street_address: String,
    #[serde(rename = "City ST ZIP")]
    pub city_state_zip: String,
     */
}

impl ContactInformation {
    pub fn new(precinct: &str) -> Self {
        ContactInformation {
            precinct: precinct.to_string(),
        }
    }
}

pub fn get_contact_information(rows: &Vec<ContactInformation>) -> Vec<&ContactInformation> {
    let iter = rows.iter();

    let precinct_rows = iter
        .skip_while(|row| row_has_precinct(row).is_none())
        .map_while(|row| row_has_precinct(row))
        .collect::<Vec<&ContactInformation>>();

    precinct_rows
}

fn row_has_precinct(row: &ContactInformation) -> Option<&ContactInformation> {
    if row.precinct.is_empty() { None } else { Some(row) }
}
