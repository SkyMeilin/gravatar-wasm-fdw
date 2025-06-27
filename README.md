# Gravatar WASM Foreign Data Wrapper

A PostgreSQL Foreign Data Wrapper (FDW) for accessing Gravatar profile data, implemented as a WebAssembly component using the [Supabase Wrappers framework](https://github.com/supabase/wrappers).

## Features

- Query Gravatar profiles by email address
- Returns profile information including display name, avatar URL, location, etc.
- Email hashing using SHA-256 (as required by Gravatar API)

## Installation

1. Build the WASM component:
```bash
cargo component build --target wasm32-unknown-unknown
```

2. The resulting WASM file will be at: `target/wasm32-unknown-unknown/debug/gravatar_fdw.wasm`

## FDW Setup

Since this FDW implementation doesn't include automatic schema import (yet), you need to manually create the foreign server and table:

```sql
-- Install Wrappers extension

create extension if not exists wrappers with schema extensions;

create foreign data wrapper wasm_wrapper
  handler wasm_fdw_handler
  validator wasm_fdw_validator;

-- Add Gravatar FDW server
create server gravatar_server
  foreign data wrapper wasm_wrapper
  options (
    fdw_package_url 'file:///gravatar_fdw.wasm', -- Use this to test from within the container
    -- fdw_package_url 'https://github.com/Automattic/gravatar-wasm-fdw/releases/download/v0.1.0/gravatar_fdw.wasm',
    fdw_package_name 'automattic:gravatar-fdw',
    fdw_package_version '0.1.0'
  );

-- Create schema and tables
create schema if not exists gravatar;

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

- **Profile not found (404)**: Returns no rows (expected for private or non-existing profiles)
- **API errors**: Returns no rows, logs error details
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

### Local Testing

Requires `docker` and `supabase` cli. See official Wasm Wrapper instructions [here](https://fdw.dev/guides/wasm-advanced/#developing-locally). 

Start the `supabase` environment with:
```
supabase init
supabase start
```

Then run `./local-dev.sh` to build and copy the resulting `*.wasm` into the container. Repeat on every change (or do `cargo watch -s ./local-dev.sh`).

Go into the SQL Editor from supabase at `http://127.0.0.1:54323/project/default/sql/1`.

Copy, paste and run the SQL from the "FDW Setup" section above.


### Building

```bash
# Development build
cargo component build --target wasm32-unknown-unknown

# Release build
cargo component build --target wasm32-unknown-unknown --release
```

## Limitations

- Requires email filters in WHERE clause (cannot scan without email)
- Only supports equality filters on email field
- No automatic schema import (yet)
- Read-only (no INSERT/UPDATE/DELETE operations)

## License

[Apache License Version 2.0](./LICENSE)