# Gravatar WASM Foreign Data Wrapper

A PostgreSQL Foreign Data Wrapper (FDW) for accessing Gravatar profile data, implemented as a WebAssembly component using the [Supabase Wrappers framework](https://github.com/supabase/wrappers).

## Features

- Query Gravatar profiles by email address
- Returns profile information including display name, avatar URL, location, etc.
- Email hashing using SHA-256 (as required by Gravatar API)
- Support for API key authentication via Supabase Vault extension
- Automatic fallback to public API when no API key is provided

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

-- Recommended: Add Gravatar FDW server (with API key from Vault)
--   First, store your API key in Vault
select vault.create_secret('your-gravatar-api-key-value', 'gravatar-api-key');
--   Then use the new vault secret's ID
create server gravatar_server
  foreign data wrapper wasm_wrapper
  options (
    fdw_package_url 'file:///gravatar_fdw.wasm', -- Use this to test from within the container
    -- fdw_package_url 'https://github.com/Automattic/gravatar-wasm-fdw/releases/download/v0.1.0/gravatar_fdw.wasm',
    fdw_package_name 'automattic:gravatar-fdw',
    fdw_package_version '0.1.0',
    api_key_id 'your-vault-secret-uuid-here'
  );

-- Alternative 1: Direct API key (not recommended for production)
-- create server gravatar_server
--   foreign data wrapper wasm_wrapper
--   options (
--     fdw_package_url 'file:///gravatar_fdw.wasm',
--     fdw_package_name 'automattic:gravatar-fdw',
--     fdw_package_version '0.1.0',
--     api_key 'your-direct-api-key-here'
--   );

-- Alternative 2: No API key (only for development, rate limited)
-- create server gravatar_server
--   foreign data wrapper wasm_wrapper
--   options (
--     fdw_package_url 'file:///gravatar_fdw.wasm',
--     fdw_package_name 'automattic:gravatar-fdw',
--     fdw_package_version '0.1.0'
--   );

-- Optional: Delete existing schema
-- drop schema gravatar cascade;

-- Create schema and tables
create schema if not exists gravatar;

CREATE FOREIGN TABLE gravatar.profiles (
  hash text,
  email text,
  display_name text,
  profile_url text,
  avatar_url text,
  avatar_alt_text text,
  location text,
  description text,
  job_title text,
  company text,
  verified_accounts jsonb,
  pronunciation text,
  pronouns text,
  timezone text,
  first_name text,
  last_name text,
  is_organization bool,
  links jsonb,
  interests jsonb,
  payments jsonb,
  contact_info jsonb,
  number_verified_accounts int,
  last_profile_edit timestamp,
  registration_date timestamp,
  json jsonb  -- Complete profile data as JSON
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
FROM gravatar.profiles
WHERE email = 'user@example.com';
```

### Query Multiple Profiles

```sql
SELECT email, display_name, company
FROM gravatar.profiles
WHERE email IN ('user1@example.com', 'user2@example.com');
```

### Get Complete Profile Data

```sql
SELECT email, attrs
FROM gravatar.profiles
WHERE email = 'user@example.com';
```

## Column Descriptions

| Column                     | Type      | Description |
|----------------------------|-----------|-------------|
| `hash`                     | text      | SHA-256 hash of the email (used by Gravatar API) |
| `email`                    | text      | Email address (added by FDW, not returned by API) |
| `display_name`             | text      | Display name |
| `profile_url`              | text      | URL to the Gravatar profile page |
| `avatar_url`               | text      | URL to the avatar image |
| `avatar_alt_text`          | text      | Alt text for the avatar image |
| `location`                 | text      | Location |
| `description`              | text      | Profile description/bio |
| `job_title`                | text      | Job title |
| `company`                  | text      | Company |
| `verified_accounts`        | jsonb     | Verified social media accounts |
| `pronunciation`            | text      | Pronunciation guide for the user's name |
| `pronouns`                 | text      | User's pronouns |
| `timezone`                 | text      | User's timezone |
| `languages`                | jsonb     | Languages spoken by the user |
| `first_name`               | text      | First name |
| `last_name`                | text      | Last name |
| `is_organization`          | bool      | Whether this is an organization profile |
| `links`                    | jsonb     | Social media and website links |
| `interests`                | jsonb     | User's interests and hobbies |
| `payments`                 | jsonb     | Payment methods and donation links |
| `contact_info`             | jsonb     | Contact information |
| `number_verified_accounts` | int       | Number of verified social media accounts |
| `last_profile_edit`        | timestamp | Date and time of last profile edit |
| `registration_date`        | timestamp | Account registration date |
| `json`                     | jsonb     | Complete profile data as returned by API |

## Error Handling

- **Profile not found (404)**: Returns no rows (expected for private or non-existing profiles)
- **API errors**: Returns no rows, logs error details
- **No email filter**: Returns empty result set with informational message
- **Rate Limit**: Returns error with details on the time to wait and how to get higher rate limits

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

To check the logs you can tweak the log levels with:
```sql
-- Set to show INFO messages in the logs
set log_min_messages to 'info';

-- Now try your query
select * from gravatar.profiles where email = 'test@example.com';
```

And then check the logs using docker `docker logs` on the database container, or checking on the SQL Editor the Logs tab for Postgres at `http://127.0.0.1:54323/project/default/logs/postgres-logs`.


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
- Any request failure implies three retries with exponential backoff.
    - This is Wrapper's default behaviour and can't be disabled.
    - Specially annoying for Rate Limit errors (HTTP 429).

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

This project is based on the [postgres-wasm-fdw](https://github.com/supabase-community/postgres-wasm-fdw) template, which is licensed under Apache 2.0. The original template code remains under Apache 2.0 license - see the [LICENSE-APACHE](LICENSE-APACHE) file for details.

The combined work is distributed under GPL v3.0, which is compatible with Apache 2.0.