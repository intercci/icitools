# tos3cf

`tos3cf` is a high-performance command-line utility written in Rust designed to automate the deployment of static assets to AWS S3 and trigger a CloudFront cache invalidation. It is optimized for developer workflows where environment-specific configurations are managed via `.env` files.

## ✨ Features

- 🚀 **Automated S3 Sync**: Performs a highly selective `aws s3 sync` with predefined exclusions to ensure only necessary files are uploaded.
- ☁️ **CloudFront Invalidation**: Automatically triggers a `/*` path invalidation to ensure changes are reflected globally immediately after sync.
- 🎨 **Fancy Terminal UI**: Provides clear, color-coded feedback and emoji-enhanced status updates for a better developer experience.
- 🛡️ **Robust Error Handling**: Gracefully handles missing directories, missing `.env` files, missing environment variables, and AWS CLI failures.

## 📋 Prerequisites

- **Rust & Cargo**: Ensure you have the Rust toolchain installed.
- **AWS CLI**: Must be installed and configured on your system.
- **AWS Credentials**: The tool requires appropriate permissions to perform `s3:Sync` and `cloudfront:CreateInvalidation`.

## 🛠️ Installation

1. Clone this repository or download the source.
2. Build the tool using Cargo:

```bash
cargo build --release
```

The binary will be located at `./target/release/tos3cf`.

## 🚀 Usage

The tool requires a folder path as an argument. This folder **must** contain a `.env` file with the required environment variables.

```bash
./tos3cf <path_to_your_folder>
```

### Environment Variables

| Variable | Description |
| :--- | :--- |
| `S3BUCKET` | The name of the target S3 bucket. |
| `CFID` | The CloudFront Distribution ID to invalidate. |
| `AWS_PROFILE` | (Optional) The AWS profile to use. If not provided, the default profile is used. |

**Example `.env` file:**

```env
S3BUCKET=my-awesome-web-assets
CFID=E1ABC2DEF3GHI4JKL
AWS_PROFILE=icci
```

## ⚙️ Default Exclusions

To prevent syncing unnecessary metadata and local development artifacts, the following paths are automatically excluded from the S3 sync:

- `.git/*`
- `.gitignore`
- `.claude`
- `.playwright-mcp/*`
- `.sisyphus/*`
- `qa-screenshots/*`
- `scripts/*`
- `data`
- `.opencode`

## ⚠️ Important Notes

- **AWS Profile**: If `AWS_PROFILE` is provided in the `.env` file, it will be used for both S3 sync and CloudFront invalidation. Otherwise, the tool uses the default AWS configuration.
- **Working Directory**: The sync operation is executed with the target folder as the working directory.
```