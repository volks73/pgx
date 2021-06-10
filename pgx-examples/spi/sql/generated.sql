-- This file is auto generated by pgx.
-- `extension_sql!()` defined SQL.
-- src/lib.rs:102


CREATE TABLE foo ();



-- src/lib.rs:5
   

CREATE TABLE spi_example (
    id serial8 not null primary key,
    title text
);

INSERT INTO spi_example (title) VALUES ('This is a test');
INSERT INTO spi_example (title) VALUES ('Hello There!');
INSERT INTO spi_example (title) VALUES ('I like pudding');




-- Enums derived via `#[derive(PostgresEnum)]`


-- Shell types for types defined by `#[derive(PostgresType)]`


-- Functions defined by `#[pg_extern]`

-- src/lib.rs:90
-- spi::spi_insert_title2
CREATE OR REPLACE FUNCTION "spi_insert_title2"(
	"title" text /* &str */
) RETURNS TABLE (
	"id" bigint /* core::option::Option<i64> */,
	"title" text /* core::option::Option<alloc::string::String> */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'spi_insert_title2';

-- src/lib.rs:81
-- spi::spi_insert_title
CREATE OR REPLACE FUNCTION "spi_insert_title"(
	"title" text /* &str */
) RETURNS bigint /* i64 */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'spi_insert_title';

-- src/lib.rs:59
-- spi::spi_query_by_id
CREATE OR REPLACE FUNCTION "spi_query_by_id"(
	"id" bigint /* i64 */
) RETURNS text /* core::option::Option<alloc::string::String> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'spi_query_by_id';

-- src/lib.rs:51
-- spi::spi_query_title
CREATE OR REPLACE FUNCTION "spi_query_title"(
	"title" text /* &str */
) RETURNS bigint /* core::option::Option<i64> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'spi_query_title';

-- src/lib.rs:46
-- spi::spi_query_random_id
CREATE OR REPLACE FUNCTION "spi_query_random_id"() RETURNS bigint /* core::option::Option<i64> */
IMMUTABLE PARALLEL SAFE STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'spi_query_random_id';

-- src/lib.rs:21
-- spi::spi_return_query
CREATE OR REPLACE FUNCTION "spi_return_query"() RETURNS TABLE (
	"oid" core::option::Option<u32> /* core::option::Option<u32> */,
	"name" text /* core::option::Option<alloc::string::String> */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'spi_return_query';


-- Types defined by `#[derive(PostgresType)]`


-- Operator classes defined by `#[derive(PostgresHash, PostgresOrd)]`
