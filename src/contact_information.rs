use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ContactInformation {
    #[serde(rename = "Precinct")]
    pub precinct: Option<String>,
    #[serde(rename = "STATUS")]
    pub status: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Preferred Email")]
    pub email_address: Option<String>,
    #[serde(rename = "Phone")]
    pub phone_number: Option<String>,
    #[serde(rename = "Street Address")]
    pub street_address: Option<String>,
    #[serde(rename = "City ST ZIP")]
    pub city_state_zip: Option<String>,
}

pub fn get_contact_information(rows: &Vec<ContactInformation>) -> Vec<&ContactInformation> {
    let iter = rows.iter();
    let iter = iter.skip(1);

    let precinct_rows = iter
        .skip_while(|row| row_has_value(row).is_none())
        .map_while(|row| row_has_value(row))
        .collect::<Vec<&ContactInformation>>();

    precinct_rows
}

fn row_has_value(row: &ContactInformation) -> Option<&ContactInformation> {
    match row.precinct {
        Some(_) => Some(row),
        _ => None,
    }
}
