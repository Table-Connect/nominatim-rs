use std::{str::FromStr, time::Duration};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

mod ident;

pub use ident::IdentificationMethod;

/// The interface for accessing a Nominatim API server.
#[derive(Debug, Clone)]
pub struct Client {
    ident: IdentificationMethod, // how to access the server
    base_url: Url,               // defaults to https://nominatim.openstreetmap.org
    client: reqwest::Client,
    /// HTTP Request Timeout [`Duration`]
    pub timeout: Duration,
}

impl Client {
    /// Create a new [`Client`] from an [`IdentificationMethod`].
    pub fn new(ident: IdentificationMethod) -> Self {
        let timeout = Duration::from_secs(10);

        Self {
            ident,
            base_url: Url::parse("https://nominatim.openstreetmap.org/").unwrap(),
            client: reqwest::ClientBuilder::new()
                .timeout(timeout)
                .build()
                .unwrap(),
            timeout,
        }
    }

    /// Set the client's internal base url for all requests.
    pub fn set_base_url<U: TryInto<Url>>(&mut self, url: U) -> Result<(), U::Error> {
        self.base_url = url.try_into()?;

        Ok(())
    }

    /// Check the status of the nominatim server.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Status/)
    pub async fn status(&self) -> Result<Status, reqwest::Error> {
        let mut url = self.base_url.join("status.php").unwrap();
        url.set_query(Some("format=json"));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Get [`Place`]s from a search query.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Search/)
    pub async fn search(&self, query: impl AsRef<str>) -> Result<Vec<Place>, reqwest::Error> {
        let mut url = self.base_url.clone();
        url.set_query(Some(&format!(
            "addressdetails=1&extratags=1&q={}&format=json&polygon_geojson=1",
            query.as_ref().replace(' ', "+")
        )));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    pub async fn search_structured(
        &self,
        params: &SearchStructuredParams,
    ) -> Result<Vec<Place>, reqwest::Error> {
        let mut url = self.base_url.clone();

        // Build the query string with structured parameters
        let mut query_parts = Vec::new();

        query_parts.push(format!("country={}", urlencoding::encode(&params.country)));
        query_parts.push(format!("state={}", urlencoding::encode(&params.state)));

        if let Some(city) = params.city.as_ref() {
            query_parts.push(format!("city={}", urlencoding::encode(city)));
        }

        if let Some(postal_code) = params.postal_code.as_ref() {
            query_parts.push(format!("postalcode={}", urlencoding::encode(postal_code)));
        }

        // Add other default parameters (addressdetails, format, etc.)
        query_parts.push("format=json".to_string());
        query_parts.push("addressdetails=true".to_string());
        query_parts.push("extratags=true".to_string());
        query_parts.push("polygon_geojson=true".to_string());

        url.set_query(Some(&query_parts.join("&")));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Generate a [`Place`] from latitude and longitude.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Reverse/)
    pub async fn reverse(
        &self,
        latitude: impl AsRef<str>,
        longitude: impl AsRef<str>,
        zoom: Option<u8>,
    ) -> Result<Place, reqwest::Error> {
        let mut url = self.base_url.join("reverse").unwrap();

        match zoom {
            Some(zoom) => {
                url.set_query(Some(&format!(
                    "addressdetails=1&extratags=1&format=json&lat={}&lon={}&zoom={}",
                    latitude.as_ref().replace(' ', ""),
                    longitude.as_ref().replace(' ', ""),
                    zoom
                )));
            }
            None => {
                url.set_query(Some(&format!(
                    "addressdetails=1&extratags=1&format=json&lat={}&lon={}",
                    latitude.as_ref().replace(' ', ""),
                    longitude.as_ref().replace(' ', ""),
                )));
            }
        }

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Return [`Place`]s from a list of OSM Node, Way, or Relations.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Lookup/)
    pub async fn lookup(&self, queries: Vec<&str>) -> Result<Vec<Place>, reqwest::Error> {
        let queries = queries.join(",");

        let mut url = self.base_url.join("lookup").unwrap();
        url.set_query(Some(&format!(
            "osm_ids={}&addressdetails=1&extratags=1&format=json",
            queries
        )));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchStructuredParams {
    pub country: String,
    pub state: String,
    pub city: Option<String>,
    pub postal_code: Option<String>,
}

/// The status of a Nominatim server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub status: usize,
    pub message: String,
    pub data_updated: Option<String>,
    pub software_version: Option<String>,
    pub database_version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GeoJsonType {
    Point,
    LineString,
    Polygon,
    MultiPolygon,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Coordinates {
    Point([f64; 2]),
    LineString(Vec<[f64; 2]>),
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoJson {
    #[serde(rename = "type")]
    pub geo_type: GeoJsonType,
    pub coordinates: Coordinates,
}

/// A location returned by the Nominatim server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Place {
    #[serde(default)]
    pub place_id: usize,
    #[serde(default)]
    pub licence: String,
    #[serde(default)]
    pub osm_type: String,
    #[serde(default)]
    pub osm_id: usize,
    #[serde(default)]
    pub boundingbox: Vec<String>,
    #[serde(default)]
    pub lat: String,
    #[serde(default)]
    pub lon: String,
    #[serde(default)]
    pub display_name: String,
    pub class: Option<String>,
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub importance: Option<f64>,
    pub icon: Option<String>,
    #[serde(default)]
    pub address: Option<Address>,
    pub extratags: Option<ExtraTags>,
    pub geojson: Option<GeoJson>,
    pub name: Option<String>,
}

/// An address for a place.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Address {
    pub city: Option<String>,
    pub state_district: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "ISO3166-2-lvl4")]
    pub iso3166_2_lvl4: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}

/// Extra metadata that a place may have.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtraTags {
    pub capital: Option<String>,
    pub website: Option<String>,
    pub wikidata: Option<String>,
    pub wikipedia: Option<String>,
    pub population: Option<String>,
}
