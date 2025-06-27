# Gravatar WASM Foreign Data Wrapper

A PostgreSQL Foreign Data Wrapper (FDW) for accessing Gravatar profile data, implemented as a WebAssembly component using the [Supabase Wrappers framework](https://github.com/supabase/wrappers).

## Features

- Query Gravatar profiles by email address
- Returns profile information including display name, avatar URL, location, etc.
- Email hashing using SHA-256 (as required by Gravatar API)
- Handles missing profiles gracefully
- Implemented as a WASM component for enhanced security and portability

## Installation

1. Build the WASM component:
```bash
cargo component build --target wasm32-unknown-unknown
```

2. The resulting WASM file will be at: `target/wasm32-unknown-unknown/debug/gravatar_fdw.wasm`

## Database Setup

Since this FDW implementation doesn't include automatic schema import, you need to manually create the foreign server and table:

### 1. Create Foreign Server

```sql
CREATE SERVER gravatar_server
FOREIGN DATA WRAPPER wasm_fdw
OPTIONS (
  fdw_package_url 'file:///path/to/gravatar_fdw.wasm',
  fdw_package_name 'automattic:gravatar-fdw',
  fdw_package_version '0.1.0',
  fdw_package_checksum 'sha256:your_checksum_here',
  api_url 'https://api.gravatar.com/v3/profiles'  -- Optional: defaults to this URL
);
```

### 2. Create Foreign Table

```sql
CREATE FOREIGN TABLE gravatar_profiles (
  hash text,
  email text,
  profile_url text,
  avatar_url text,
  avatar_alt_text text,
  display_name text,
  pronouns text,
  location text,
  job_title text,
  company text,
  description text,
  verified_accounts jsonb,
  attrs jsonb  -- Complete profile data as JSON
)
SERVER gravatar_server
OPTIONS (
  table 'profiles'
);
```

## Usage

The FDW requires an email filter in your queries. You cannot scan all profiles without specifying an email:

### Query Single Profile

```sql
SELECT display_name, avatar_url, location 
FROM gravatar_profiles 
WHERE email = 'user@example.com';
```

### Query Multiple Profiles

```sql
SELECT email, display_name, company
FROM gravatar_profiles 
WHERE email IN ('user1@example.com', 'user2@example.com');
```

### Get Complete Profile Data

```sql
SELECT email, attrs
FROM gravatar_profiles 
WHERE email = 'user@example.com';
```

## Column Descriptions

| Column | Type | Description |
|--------|------|-------------|
| `hash` | text | SHA-256 hash of the email (used by Gravatar API) |
| `email` | text | Email address (added by FDW, not returned by API) |
| `profile_url` | text | URL to the Gravatar profile page |
| `avatar_url` | text | URL to the avatar image |
| `avatar_alt_text` | text | Alt text for the avatar image |
| `display_name` | text | Display name |
| `pronouns` | text | User's pronouns |
| `location` | text | Location |
| `job_title` | text | Job title |
| `company` | text | Company |
| `description` | text | Profile description/bio |
| `verified_accounts` | jsonb | Verified social media accounts |
| `attrs` | jsonb | Complete profile data as returned by API |

## Error Handling

- **Profile not found (404)**: Returns a minimal record with email and hash only
- **API errors**: Returns a minimal record with email and hash, logs error
- **No email filter**: Returns empty result set with informational message

## Development

### Project Structure

```bash
├── src/
│   ├── lib.rs              # Main FDW implementation
│   └── bindings.rs         # Generated WIT bindings
├── wit/
│   └── world.wit           # WIT world definition
├── supabase-wrappers-wit/  # Supabase WIT interface definitions
└── Cargo.toml
```

### Building

```bash
# Development build
cargo component build --target wasm32-unknown-unknown

# Release build
cargo component build --target wasm32-unknown-unknown --release
```

### Dependencies

- `wit-bindgen-rt`: WIT runtime for Rust
- `serde_json`: JSON parsing
- `sha2`: SHA-256 hashing for email addresses

## Limitations

- Requires email filters in WHERE clause (cannot scan without email)
- Only supports equality filters on email field
- Uses Supabase Wrappers v0.1.0 WIT interface (no automatic schema import)
- Read-only (no INSERT/UPDATE/DELETE operations)

## License

[Apache License Version 2.0](./LICENSE)