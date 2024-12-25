# Backend Payment System

This project is a backend payment system written in Rust. It includes user authentication, transaction management, and audit logging.

## Prerequisites

Before you begin, ensure you have the following installed on your machine:

- [Rust](https://www.rust-lang.org/tools/install)
- [Docker (Optional)](https://docs.docker.com/get-docker/)
- [PostgreSQL](https://www.postgresql.org/download/)
- [Sqlx](https://crates.io/sqlx)

## Setup

### 1. Clone the Repository

Clone the repository to your local machine using the following command:

```sh
git clone https://github.com/artech-git/backend-payment-system.git
cd backend-payment-system
```

### 2. Set Up Environment Variables
Create a .env file in the root directory and add the necessary environment variables. Here is an example:
```bash
DATABASE_URL=postgres://user:password@localhost/dbname
JWT_SECRET=your_secret_key // your jwt secret key
```
Please setup these keys as your enviroment variable based upon your shell

for linux or macos you may have to call `source` but for windows it's different
```bash
$ source .env
```

### 3. Install sqlx on your machine
We need sqlx to run this project, you can out detailed documentation [here](https://crates.io/crates/sqlx-cli) 

```bash
$ cargo install sqlx-cli
```

once this complete then run following command to perform database migration, Note: your `$DATABASE_URL` must be set before running below command  

```bash
sqlx migrate run
```

### 4. Running your project

To run your project you can run this through cargo or Docker but If you are testing I would recommend to use only `cargo ` locally

```bash
cargo r 
```

you should also see something like this below and a `app.log` file being created where the same process is being logged as well

```bash 
warning: `backend-payment-system` (bin "backend-payment-system") generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.47s
     Running `target/debug/backend-payment-system`
2024-12-25T08:33:35.458704Z  INFO sqlx::postgres::notice: relation "_sqlx_migrations" already exists, skipping
2024-12-25T08:33:35.698193Z ERROR backend_payment_system: Failed to run migrations: Failed to run migrations: migration 1 was previously applied but has been modified
2024-12-25T08:33:35.698317Z  INFO backend_payment_system: Connected to database
2024-12-25T08:33:35.698662Z  INFO backend_payment_system: Listening on port: 3000
2024-12-25T08:33:35.700843Z  INFO backend_payment_system: Routes constructed successfully
```

## API Routes


### 1. For registering a new user

For a detailed API layout I have generated a OpenAPI specs in `dodo payments.openapi.json` file
To register a new user simply make a call to `\v1\auth\register` you need to provide user data as input

```bash
curl --location --request POST 'http://localhost:3000/v1/auth/register' \
--header 'Content-Type: application/json' \
--data-raw '{
    "email": "hari@gmail.com",
    "password": "HariOne123!",
    "full_name": "Hari singh"
}'
```

you should get something like this as output

```bash
{
    "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI4ODI0MTAxNS04ODdkLTQxYzMtOTA3ZS1kMmZjMTBkYjg4MDUiLCJleHAiOjE3MzUxMTY2NzMsImlhdCI6MTczNTExNTc3M30.NbVoZDjWUQSU5_Tr4lTUYPH33iJ6ddC_I5BaBIFI-JI",
    "refresh_token": "b699cbe4-831c-4cca-ae77-3451ac8cc1ab",
    "user_uid": "88241015-887d-41c3-907e-d2fc10db8805"
}
```

### 2. Checking a user 

To check about a new or existing user, paste the `access_token` as a `Authorization` param

```bash
curl --location --request GET 'http://localhost:3000/v1/users/uid' \
--header 'Authorization: eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI4ODI0MTAxNS04ODdkLTQxYzMtOTA3ZS1kMmZjMTBkYjg4MDUiLCJleHAiOjE3MzUxMTY2NzMsImlhdCI6MTczNTExNTc3M30.NbVoZDjWUQSU5_Tr4lTUYPH33iJ6ddC_I5BaBIFI-JI' \
--header 'User-Agent: Apidog/1.0.0 (https://apidog.com)'
```
you would get something like this as output

```bash
{
    "id": "88241015-887d-41c3-907e-d2fc10db8805",
    "email": "hari@gmail.com",
    "full_name": "Hari singh",
    "balance": "0",
    "status": "active",
    "created_at": "2024-12-25T08:36:13.793829Z",
    "updated_at": "2024-12-25T08:36:13.793829Z"
}
```

### 3. Depositing amount to user

To make a deposit to user account, you need `Authorization` to be set you need to provide `email`, `full_name` and `amount` you wish to transfer

```bash
curl --location --request POST 'http://localhost:3000/v1/users/deposit' \
--header 'Authorization: eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI4ODI0MTAxNS04ODdkLTQxYzMtOTA3ZS1kMmZjMTBkYjg4MDUiLCJleHAiOjE3MzUxMTY2NzMsImlhdCI6MTczNTExNTc3M30.NbVoZDjWUQSU5_Tr4lTUYPH33iJ6ddC_I5BaBIFI-JI' \
--header 'User-Agent: Apidog/1.0.0 (https://apidog.com)' \
--header 'Content-Type: application/json' \
--data-raw '{
    "email": "hari@gmail.com",
    "full_name": "hari om",
    "amount": "800"
}'
```
you should get something like this as output
```bash
User balance updated successfully. New balance: 800.0000
```

### 4. Make a transaction 

To make a transfer from a user A to user B you need both users ID and amount you wish to transfer
and also `Authentication` token to be set 

```bash
curl --location --request POST 'http://localhost:3000/v1/tx/transfer' \
--header 'Authorization: eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI4ODI0MTAxNS04ODdkLTQxYzMtOTA3ZS1kMmZjMTBkYjg4MDUiLCJleHAiOjE3MzUxMTY2NzMsImlhdCI6MTczNTExNTc3M30.NbVoZDjWUQSU5_Tr4lTUYPH33iJ6ddC_I5BaBIFI-JI' \
--header 'User-Agent: Apidog/1.0.0 (https://apidog.com)' \
--header 'Content-Type: application/json' \
--data-raw '{
    "sender_id": "88241015-887d-41c3-907e-d2fc10db8805",
    "receiver_id": "efd3ff9d-e5a7-4f04-bd67-5376604eafe5",
    "amount": "100", 
    "description" : "personel transfer"
}'
```
You should see something like this as a response

```bash
Transaction successful id: 6dbe6907-5fc3-4df1-a7e5-968f8fef87a3
```