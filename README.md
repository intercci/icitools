# icitools

A collection of command-line utilities built around the [iciawsaid](https://github.com/iciawsaid) crates ecosystem — Rust libraries for AWS infrastructure tooling.

Each subdirectory is an independent tool you can compile and install on its own.

## Tools

| Tool | Language | Description |
|------|----------|-------------|
| [dynamo_load](dynamo_load/) | Rust | Load JSON data into Amazon DynamoDB with batch writes and exponential backoff retry. |
| [gen_routes](gen_routes/) | Rust | Generate Lambda API Gateway route handlers from project structure. |
| [gen_token](gen_token/) | Rust | Generate secure tokens using bcrypt hashing and PASETOT encryption. |
| [lamb_policy](lamb_policy/) | Python | Clean up duplicate permissions in AWS Lambda function policies. |
| [substr](substr/) | Rust | String substring extraction utility. |
| [tos3cf](tos3cf/) | Rust | Sync static assets to S3 and trigger CloudFront cache invalidation with .env config. |
| [upver](upver/) | Rust | Semantic version parsing and manipulation tool. |

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable) **or** [Python 3.13+](https://www.python.org/downloads/) for `lamb_policy`
- An AWS account with appropriate credentials configured

### Compile & Install

Every tool is self-contained. Navigate to the directory and build:

```sh
cd <tool_name>
cargo build --release
cargo install --path .
```

For the Python tool (`lamb_policy`), run it directly via [uv](https://docs.astral.sh/uv/):

```sh
cd lamb_policy
uv run check_policy.py <function-name>
```

## License

This project is licensed under the [MIT License](LICENSE).
