// Copyright 2025 Automattic
//
// This file is part of Gravatar Wasm Foreign Data Wrapper which is licensed under
// the GNU General Public License v3.0.
#[allow(warnings)]
mod bindings;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use bindings::{
    exports::supabase::wrappers::routines::Guest,
    supabase::wrappers::{
        http,
        types::{Cell, Context, FdwError, FdwResult, OptionsType, Row, TypeOid, Value},
        utils,
    },
};

#[derive(Debug, Default)]
struct GravatarFdw {
    base_url: String,
    scanned_profiles: Vec<JsonValue>,
    scan_index: usize,
}

// pointer for the static FDW instance
static mut INSTANCE: *mut GravatarFdw = std::ptr::null_mut::<GravatarFdw>();

impl GravatarFdw {
    const PROFILES_OBJECT: &'static str = "profiles";

    // initialise FDW instance
    fn init_instance() {
        let instance = Self::default();
        unsafe {
            INSTANCE = Box::leak(Box::new(instance));
        }
    }

    fn this_mut() -> &'static mut Self {
        unsafe { &mut (*INSTANCE) }
    }

    // Hash email using SHA-256
    fn hash_email(email: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(email.trim().to_lowercase().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // Build URL for gravatar profile
    fn build_url(&self, email: &str) -> String {
        let hash = Self::hash_email(email);
        format!("{}/{}", self.base_url, hash)
    }
}

impl Guest for GravatarFdw {
    fn host_version_requirement() -> String {
        // semver expression for Wasm FDW host version requirement
        // ref: https://docs.rs/semver/latest/semver/enum.Op.html
        "^0.1.0".to_string()
    }

    fn init(ctx: &Context) -> FdwResult {
        Self::init_instance();
        let this = Self::this_mut();

        let opts = ctx.get_options(OptionsType::Server);
        this.base_url = opts.require_or("api_url", "https://api.gravatar.com/v3/profiles");

        utils::report_info(&format!("Gravatar FDW initialized with base URL: {}", this.base_url));

        Ok(())
    }

    fn begin_scan(ctx: &Context) -> FdwResult {
        let this = Self::this_mut();

        // Clear previous results
        this.scanned_profiles.clear();
        this.scan_index = 0;

        let opts = ctx.get_options(OptionsType::Table);
        let table = opts.require_or("table", Self::PROFILES_OBJECT);

        if table != Self::PROFILES_OBJECT {
            return Err(format!("Unsupported table '{}'. Only 'profiles' is supported.", table));
        }

        // Look for email filters in quals
        let mut emails_to_fetch = Vec::new();
        for qual in ctx.get_quals() {
            if qual.field() == "email" && qual.operator() == "=" {
                if let Value::Cell(Cell::String(email)) = qual.value() {
                    emails_to_fetch.push(email);
                }
            }
        }

        // If no email filter provided, we can't fetch profiles
        if emails_to_fetch.is_empty() {
            utils::report_info("No email filters provided. Gravatar FDW requires email = 'email@example.com' in WHERE clause");
            return Ok(());
        }

        // Fetch profiles for each email
        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        utils::report_info(&format!("Using User-Agent: {}", user_agent));
        let headers: Vec<(String, String)> = vec![
            ("user-agent".to_owned(), user_agent),
            ("accept".to_owned(), "application/json".to_owned()),
        ];

        for email in emails_to_fetch {
            let url = this.build_url(&email);
            
            let req = http::Request {
                method: http::Method::Get,
                url,
                headers: headers.clone(),
                body: String::default(),
            };
            
            let resp = http::get(&req)?;
            
            if resp.status_code == 200 {
                // Parse successful response
                let mut profile: JsonValue = serde_json::from_str(&resp.body)
                    .map_err(|e| format!("Failed to parse JSON response: {}", e))?;
                
                // Add email to the response since API doesn't return it
                if let JsonValue::Object(ref mut map) = profile {
                    map.insert("email".to_string(), JsonValue::String(email.clone()));
                }
                
                this.scanned_profiles.push(profile);
            } else {
                // Handle 404 (expected for private or non-existing profiles) and generic API errors
                // by skipping this email - no row will be returned for failed lookups
                if resp.status_code == 404 {
                    utils::report_info(&format!("Profile not found for email: {}", email));
                } else {
                    utils::report_info(&format!("HTTP error {} for email {}: {}", resp.status_code, email, resp.body));
                }
            }
        }

        utils::report_info(&format!("Found {} profiles", this.scanned_profiles.len()));

        Ok(())
    }

    fn iter_scan(ctx: &Context, row: &Row) -> Result<Option<u32>, FdwError> {
        let this = Self::this_mut();

        if this.scan_index >= this.scanned_profiles.len() {
            return Ok(None);
        }

        let profile = &this.scanned_profiles[this.scan_index];
        
        for tgt_col in ctx.get_columns() {
            let tgt_col_name = tgt_col.name();
            let cell = match tgt_col_name.as_str() {
                "hash" => profile.get("hash").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "email" => profile.get("email").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "profile_url" => profile.get("profile_url").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "avatar_url" => profile.get("avatar_url").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "avatar_alt_text" => profile.get("avatar_alt_text").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "display_name" => profile.get("display_name").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "pronouns" => profile.get("pronouns").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "location" => profile.get("location").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "job_title" => profile.get("job_title").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "company" => profile.get("company").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "description" => profile.get("description").and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                "verified_accounts" => profile.get("verified_accounts").map(|v| Cell::Json(v.to_string())),
                "attrs" => Some(Cell::Json(profile.to_string())),
                _ => {
                    // For unknown columns, try to get the value directly
                    match tgt_col.type_oid() {
                        TypeOid::Bool => profile.get(&tgt_col_name).and_then(|v| v.as_bool()).map(Cell::Bool),
                        TypeOid::String => profile.get(&tgt_col_name).and_then(|v| v.as_str()).map(|s| Cell::String(s.to_string())),
                        TypeOid::I32 => profile.get(&tgt_col_name).and_then(|v| v.as_i64()).map(|i| Cell::I32(i as i32)),
                        TypeOid::I64 => profile.get(&tgt_col_name).and_then(|v| v.as_i64()).map(Cell::I64),
                        TypeOid::Json => profile.get(&tgt_col_name).map(|v| Cell::Json(v.to_string())),
                        _ => None,
                    }
                }
            };

            row.push(cell.as_ref());
        }

        this.scan_index += 1;

        Ok(Some(0))
    }

    fn re_scan(_ctx: &Context) -> FdwResult {
        let this = Self::this_mut();
        this.scan_index = 0;
        Ok(())
    }

    fn end_scan(_ctx: &Context) -> FdwResult {
        let this = Self::this_mut();
        this.scanned_profiles.clear();
        this.scan_index = 0;
        Ok(())
    }

    fn begin_modify(_ctx: &Context) -> FdwResult {
        Err("modify on foreign table is not supported".to_owned())
    }

    fn insert(_ctx: &Context, _row: &Row) -> FdwResult {
        Ok(())
    }

    fn update(_ctx: &Context, _rowid: Cell, _row: &Row) -> FdwResult {
        Ok(())
    }

    fn delete(_ctx: &Context, _rowid: Cell) -> FdwResult {
        Ok(())
    }

    fn end_modify(_ctx: &Context) -> FdwResult {
        Ok(())
    }

}

bindings::export!(GravatarFdw with_types_in bindings);
