use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

// const AWS_REGION: &str = "eu-north-1";
const AUTH_LAMBDA: &str = "iciauth";
const FN_PFX: &str = "pub async fn ";

fn snake2camel(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

fn split_paths(path: &str) -> Vec<String> {
    Regex::new(r"[/\\]")
        .unwrap()
        .split(path)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn current_time() -> String {
    let now = chrono_lite_now();
    format!("{} UTC", now)
}

fn chrono_lite_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let days = secs / 86400;
    let years = (days / 365) + 1970;
    let remaining_days = days % 365;
    let month_day = remaining_days / 30;
    let day = remaining_days % 30;
    let day_secs = secs % 86400;
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;
    format!(
        "{:02}-{:02}-{:02} {:02}:{:02}:{:02}",
        years % 100,
        month_day + 1,
        day + 1,
        hour,
        minute,
        second
    )
}

#[derive(Debug, Clone)]
struct Route {
    handler_paths: Vec<String>,
    route_path: String,
    func_name: String,
    handler: String,
}

impl Route {
    fn fpath_to_hpaths(fpath: &str) -> Vec<String> {
        let ss = split_paths(fpath);
        if let Some(idx) = ss.iter().position(|s| s == "src") {
            let mut result = ss[idx + 1..].to_vec();
            if let Some(last) = result.last_mut() {
                *last = last.replace(".rs", "");
            }
            result
        } else {
            vec![]
        }
    }

    fn new(fpath: &str, rpath: &str, func_name: &str) -> Self {
        Route {
            handler_paths: Self::fpath_to_hpaths(fpath),
            route_path: rpath.to_string(),
            func_name: func_name.to_string(),
            handler: format!("{}Handler", snake2camel(func_name)),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenApiSchema {
    #[serde(rename = "openapi")]
    openapi: String,
    info: Info,
    components: Components,
    paths: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct Info {
    title: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct Components {
    #[serde(rename = "securitySchemes")]
    security_schemes: SecuritySchemes,
}

#[derive(Debug, Serialize)]
struct SecuritySchemes {
    #[serde(rename = "ici-auth-lambda")]
    ici_auth_lambda: SecurityScheme,
}

#[derive(Debug, Serialize)]
struct SecurityScheme {
    #[serde(rename = "type")]
    r#type: String,
    name: String,
    #[serde(rename = "in")]
    in_loc: String,
    #[serde(rename = "x-amazon-apigateway-authorizer")]
    authorizer: Authorizer,
}

#[derive(Debug, Serialize)]
struct Authorizer {
    #[serde(rename = "identitySource")]
    identity_source: String,
    #[serde(rename = "authorizerUri")]
    authorizer_uri: String,
    #[serde(rename = "authorizerPayloadFormatVersion")]
    authorizer_payload_format_version: String,
    #[serde(rename = "authorizerResultTtlInSeconds")]
    authorizer_result_ttl_in_seconds: i32,
    #[serde(rename = "type")]
    r#type: String,
    #[serde(rename = "enableSimpleResponses")]
    enable_simple_responses: bool,
}

fn base_schema(title: &str, aws_region: &str, aws_account: &str) -> OpenApiSchema {
    OpenApiSchema {
        openapi: "3.0.1".to_string(),
        info: Info {
            title: title.to_string(),
            version: current_time(),
        },
        components: Components {
            security_schemes: SecuritySchemes {
                ici_auth_lambda: SecurityScheme {
                    r#type: "apiKey".to_string(),
                    name: "Cookie".to_string(),
                    in_loc: "header".to_string(),
                    authorizer: Authorizer {
                        identity_source: "$request.header.Cookie".to_string(),
                        authorizer_uri: format!(
                            "arn:aws:apigateway:{}:lambda:path/2015-03-31/functions/arn:aws:lambda:{}:{}:function:{}/invocations",
                            aws_region, aws_region, aws_account, AUTH_LAMBDA
                        ),
                        authorizer_payload_format_version: "2.0".to_string(),
                        authorizer_result_ttl_in_seconds: 0,
                        r#type: "request".to_string(),
                        enable_simple_responses: true,
                    },
                },
            },
        },
        paths: HashMap::new(),
    }
}

struct BaseClass {
    path: String,
    appid: String,
}

impl BaseClass {
    fn new(path: &str) -> Self {
        let parts: Vec<&str> = Regex::new(r"[/\\]")
            .unwrap()
            .split(path)
            .filter(|s| !s.is_empty())
            .collect();
        let appid = parts.last().unwrap_or(&"").to_string();
        BaseClass {
            path: path.to_string(),
            appid,
        }
    }

    fn inspect_and_collect(&self, fpath: &str) -> Vec<Route> {
        let mut routes = Vec::new();
        let content = fs::read_to_string(fpath).unwrap_or_default();
        let mut route_path = String::new();

        for line in content.lines() {
            if line.starts_with("#[route(\"") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        route_path = line[start + 1..start + 1 + end].to_string();
                    }
                }
            } else if line.starts_with(FN_PFX) {
                if !route_path.is_empty() {
                    let func_name = line[FN_PFX.len()..]
                        .split('(')
                        .next()
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    if !func_name.is_empty() {
                        routes.push(Route::new(fpath, &route_path, &func_name));
                    }
                    route_path.clear();
                }
            }
        }
        routes
    }

    fn search_handle_files_to_collect_routes(&self) -> Vec<Route> {
        let mut routes = Vec::new();
        for entry in WalkDir::new(&self.path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                println!("Inspect file {:?}", path.to_str());
                routes.extend(self.inspect_and_collect(path.to_str().unwrap_or("")));
            }
        }
        routes
    }
}

struct LambRouteGenerator {
    base: BaseClass,
    handlers: serde_json::Value,
}

impl LambRouteGenerator {
    fn new(path: &str) -> Self {
        LambRouteGenerator {
            base: BaseClass::new(path),
            handlers: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    fn graft_handler(&mut self, route: &Route) {
        let handler = &route.handler;
        let paths = &route.handler_paths;

        if paths.is_empty() {
            return;
        }

        let mut obj = self.handlers.as_object_mut().unwrap();

        for path in paths.iter().take(paths.len() - 1) {
            let entry = obj
                .entry(path.clone())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            *entry = entry.clone();
            obj = entry.as_object_mut().unwrap();
        }

        let leaf = paths.last().unwrap();
        let leaf_entry = obj
            .entry(leaf.clone())
            .or_insert_with(|| serde_json::Value::Array(vec![]));

        if let Some(arr) = leaf_entry.as_array_mut() {
            arr.push(serde_json::Value::String(handler.clone()));
        }
    }

    fn build_routes(&self) -> (String, String) {
        let mut uses = vec!["use crate::".to_string()];
        let mut adds = vec!["pub fn add_routes(router: &mut Router) {".to_string()];

        fn step_into(
            uses: &mut Vec<String>,
            adds: &mut Vec<String>,
            node: &serde_json::Map<String, serde_json::Value>,
        ) {
            for (k, v) in node {
                if let Some(obj) = v.as_object() {
                    uses.push(format!("{}::{{", k));
                    step_into(uses, adds, obj);
                    uses.push("},".to_string());
                } else if let Some(arr) = v.as_array() {
                    uses.push(format!("{}::", k));
                    let handlers: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();

                    if handlers.len() > 1 {
                        uses.push(format!("{{{},}},", handlers.join(", ")));
                    } else {
                        uses.push(format!("{},", handlers.join(", ")));
                    }

                    for s in &handlers {
                        adds.push(format!(
                            "    router.add_route({}::get_key(), Box::new({}));",
                            s, s
                        ));
                    }
                }
            }
        }

        if let Some(obj) = self.handlers.as_object() {
            step_into(&mut uses, &mut adds, obj);
        }

        if let Some(last) = uses.last_mut() {
            *last = last.replace(',', ";");
        }
        adds.push("}".to_string());

        let uses_str = uses.join("\n");
        let adds_str = adds.join("\n");
        (uses_str, adds_str)
    }

    fn save(&self, uses: &str, adds: &str) -> std::io::Result<()> {
        let fp = Path::new(&self.base.path).join("src").join("routes.rs");
        let content = format!("use iciaws_router::router::Router;\n\n{}\n\n{}", uses, adds);
        fs::write(fp, content)
    }

    fn gen(&self) {
        let routes = self.base.search_handle_files_to_collect_routes();

        let mut gen = LambRouteGenerator::new(&self.base.path);
        for r in &routes {
            gen.graft_handler(r);
        }

        if !gen.handlers.as_object().map_or(false, |m| m.is_empty()) {
            let (uses, adds) = gen.build_routes();
            let _ = gen.save(&uses, &adds);
        }
    }
}

struct ApiGatewaySchemaGenerator {
    base: BaseClass,
    schema: OpenApiSchema,
    schema_filename: String,
    aws_region: String,
    aws_account: String,
}

impl ApiGatewaySchemaGenerator {
    fn new(path: &str, aws_region: &str, aws_account: &str) -> Self {
        let base = BaseClass::new(path);
        let title = format!("{}-API", base.appid);
        let schema_filename = format!("{}-openapi-schema.json", title);
        let schema = base_schema(&title, aws_region, aws_account);

        ApiGatewaySchemaGenerator {
            base,
            schema,
            schema_filename,
            aws_region: aws_region.to_string(),
            aws_account: aws_account.to_string(),
        }
    }

    fn split_route(route: &str) -> (String, String) {
        if let Some(n) = route.find('/') {
            let method = route[..n].trim().to_string();
            (method, route[n..].to_string())
        } else {
            (route.trim().to_string(), "/".to_string())
        }
    }

    fn make_integration(&self, _method: &str, path: &str) -> serde_json::Value {
        let mut rd = serde_json::json!({
            "responses": {"default": {}},
            "security": [{"ici-auth-lambda": []}],
            "x-amazon-apigateway-integration": {
                "payloadFormatVersion": "2.0",
                "type": "aws_proxy",
                "httpMethod": "POST",
                "uri": format!(
                    "arn:aws:apigateway:{}:lambda:path/2015-03-31/functions/arn:aws:lambda:{}:{}:function:{}/invocations",
                    self.aws_region, self.aws_region, self.aws_account, self.base.appid
                ),
                "connectionType": "INTERNET",
            },
        });

        let path_lower = path.to_lowercase();
        if path_lower.contains("login")
            || path_lower.contains("register")
            || path_lower.contains("signin")
            || path_lower.contains("version")
        {
            rd.as_object_mut().unwrap().remove("security");
        }
        rd
    }

    fn make_parameters(&self, path: &str) -> serde_json::Value {
        let re = Regex::new(r"\{([\w_]+)\}").unwrap();
        let params: Vec<serde_json::Value> = re
            .captures_iter(path)
            .map(|caps| {
                serde_json::json!({
                    "name": caps.get(1).map_or("", |m| m.as_str()),
                    "in": "path",
                    "required": true,
                    "schema": {"type": "string"},
                })
            })
            .collect();
        serde_json::Value::Array(params)
    }

    fn gen(&self) {
        let routes = self.base.search_handle_files_to_collect_routes();

        let mut gen =
            ApiGatewaySchemaGenerator::new(&self.base.path, &self.aws_region, &self.aws_account);

        let mut updates: Vec<(String, String, serde_json::Value, Option<serde_json::Value>)> =
            Vec::new();

        for r in &routes {
            let (method, path) = Self::split_route(&r.route_path);
            let integration = gen.make_integration(&method, &path);
            let params = if path.contains('{') {
                Some(gen.make_parameters(&path))
            } else {
                None
            };
            updates.push((path.clone(), method.to_lowercase(), integration, params));
        }

        for (path, method, integration, params) in updates {
            let path_entry = gen.schema.paths.entry(path).or_insert_with(HashMap::new);
            path_entry.insert(method, integration);
            if let Some(p) = params {
                path_entry.insert("parameters".to_string(), p);
            }
        }
        gen.save_schema();
    }

    fn save_schema(&self) {
        let fp = Path::new(&self.base.path).join(&self.schema_filename);
        let content = serde_json::to_string_pretty(&self.schema).unwrap_or_default();
        let _ = fs::write(fp, content);
    }
}

struct TemplatePathsGenerator {
    base: BaseClass,
    aws_region: String,
    aws_account: String,
}

impl TemplatePathsGenerator {
    fn new(path: &str, aws_region: &str, aws_account: &str) -> Self {
        TemplatePathsGenerator {
            base: BaseClass::new(path),
            aws_region: aws_region.to_string(),
            aws_account: aws_account.to_string(),
        }
    }

    fn load_template(&self) -> std::io::Result<Vec<String>> {
        let fp = Path::new(&self.base.path).join("template-local.yaml");
        fs::read_to_string(fp)?
            .lines()
            .map(|l| Ok(l.to_string()))
            .collect()
    }

    fn save_template(&self, lines: &[String]) -> std::io::Result<()> {
        let fp = Path::new(&self.base.path).join("template-local.yaml");
        let content = lines.join("\n");
        fs::write(fp, content)
    }

    fn till_events(&self, lines: Vec<String>) -> Vec<String> {
        let mut result = Vec::new();
        for line in lines {
            result.push(line.trim_end_matches('\n').to_string());
            if line.trim() == "Events:" {
                break;
            }
        }
        result
    }

    fn gen(&self) {
        let routes = self.base.search_handle_files_to_collect_routes();
        let lines = self.load_template().unwrap_or_default();
        let mut reserved = self.till_events(lines);

        for r in &routes {
            let (method, path) = ApiGatewaySchemaGenerator::split_route(&r.route_path);
            reserved.push(format!("        {}:", r.handler));
            reserved.push("          Type: Api".to_string());
            reserved.push("          Properties:".to_string());
            reserved.push(format!("            Path: {}", path));
            reserved.push(format!("            Method: {}", method));
        }

        let _ = self.save_template(&reserved);
    }
}

pub struct Generator {
    project_folder: String,
    aws_region: String,
    aws_account: String,
}

impl Generator {
    pub fn new(folder: &str) -> Self {
        let aws_region = env::var("AWS_REGION").unwrap_or_else(|_| "eu-north-1".to_string());
        let aws_account = env::var("AWS_ACCOUNT").unwrap_or_else(|_| "".to_string());

        Generator {
            project_folder: folder.to_string(),
            aws_region,
            aws_account,
        }
    }

    pub fn run(&self, command: &str) {
        match command {
            "all" => {
                self.gen_routes();
                self.gen_openapi();
                self.gen_template();
            }
            "routes" => self.gen_routes(),
            "openapi" => self.gen_openapi(),
            "template" => self.gen_template(),
            _ => eprintln!("Unknown command: {}", command),
        }
    }

    pub fn gen_routes(&self) {
        LambRouteGenerator::new(&self.project_folder).gen();
    }

    pub fn gen_openapi(&self) {
        ApiGatewaySchemaGenerator::new(&self.project_folder, &self.aws_region, &self.aws_account)
            .gen();
    }

    pub fn gen_template(&self) {
        TemplatePathsGenerator::new(&self.project_folder, &self.aws_region, &self.aws_account)
            .gen();
    }
}

pub fn run() {
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "gen_lamb_routes")]
    #[command(about = "Generate routes and API Gateway configs for Lambda", long_about = None)]
    struct Args {
        #[arg(help = "Command: routes, openapi, template, all")]
        command: String,

        #[arg(help = "Project folder")]
        folder: String,
    }

    dotenv::dotenv().ok();

    let args = Args::parse();
    Generator::new(&args.folder).run(&args.command);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_snake2camel() {
        assert_eq!(snake2camel("get_user"), "GetUser");
        assert_eq!(snake2camel("hello_world_test"), "HelloWorldTest");
        assert_eq!(snake2camel("single"), "Single");
    }

    #[test]
    fn test_split_paths() {
        assert_eq!(
            split_paths("src/handlers/auths.rs"),
            vec!["src", "handlers", "auths.rs"]
        );
        assert_eq!(
            split_paths("src\\handlers\\auths.rs"),
            vec!["src", "handlers", "auths.rs"]
        );
        assert_eq!(split_paths("a/b/c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_route_fpath_to_hpaths() {
        assert_eq!(
            Route::fpath_to_hpaths("packman_api/src/handlers/auths.rs"),
            vec!["handlers", "auths"]
        );
        assert_eq!(
            Route::fpath_to_hpaths("project/src/routes/users.rs"),
            vec!["routes", "users"]
        );
    }

    #[test]
    fn test_route_new() {
        let route = Route::new("src/handlers/auths.rs", "GET /auth/login", "get_login");
        assert_eq!(route.handler_paths, vec!["handlers", "auths"]);
        assert_eq!(route.route_path, "GET /auth/login");
        assert_eq!(route.func_name, "get_login");
        assert_eq!(route.handler, "GetLoginHandler");
    }

    #[test]
    fn test_split_route() {
        assert_eq!(
            ApiGatewaySchemaGenerator::split_route("GET /users"),
            ("GET".to_string(), "/users".to_string())
        );
        assert_eq!(
            ApiGatewaySchemaGenerator::split_route("POST /api/login"),
            ("POST".to_string(), "/api/login".to_string())
        );
    }

    #[test]
    fn test_base_class_appid() {
        let base = BaseClass::new("/path/to/my_project");
        assert_eq!(base.appid, "my_project");
    }

    #[test]
    fn test_api_gateway_make_integration_with_auth() {
        let gen = ApiGatewaySchemaGenerator::new("/test", "eu-west-1", "123456789");
        let integration = gen.make_integration("GET", "/users");
        assert!(integration.get("security").is_some());
    }

    #[test]
    fn test_api_gateway_make_integration_no_auth() {
        let gen = ApiGatewaySchemaGenerator::new("/test", "eu-west-1", "123456789");

        let integration = gen.make_integration("GET", "/login");
        assert!(integration.get("security").is_none());

        let integration = gen.make_integration("POST", "/register");
        assert!(integration.get("security").is_none());

        let integration = gen.make_integration("GET", "/api/signin");
        assert!(integration.get("security").is_none());
    }

    #[test]
    fn test_make_parameters() {
        let gen = ApiGatewaySchemaGenerator::new("/test", "eu-west-1", "123456789");
        let params = gen.make_parameters("/users/{id}/posts/{post_id}");

        let arr = params.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("name").unwrap(), "id");
        assert_eq!(arr[1].get("name").unwrap(), "post_id");
    }

    #[test]
    fn test_lamb_route_generator_build_routes() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        fs::create_dir_all(project_path.join("src")).unwrap();

        let mut gen = LambRouteGenerator::new(project_path.to_str().unwrap());
        gen.graft_handler(&Route::new(
            "src/handlers/auths.rs",
            "GET /login",
            "get_login",
        ));
        gen.graft_handler(&Route::new(
            "src/handlers/users.rs",
            "GET /users",
            "get_users",
        ));

        let (uses, adds) = gen.build_routes();

        assert!(uses.contains("handlers::"));
        assert!(adds.contains("pub fn add_routes"));
    }

    #[test]
    fn test_till_events() {
        let gen = TemplatePathsGenerator::new("/test", "eu-west-1", "123456");

        let lines = vec![
            "Resources:".to_string(),
            "  MyFunction:".to_string(),
            "    Type: AWS::Serverless::Function".to_string(),
            "Events:".to_string(),
            "    ApiEvent:".to_string(),
        ];

        let result = gen.till_events(lines);

        assert_eq!(result.len(), 4);
        assert_eq!(result[3], "Events:");
    }

    #[test]
    fn test_generator_all_commands() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        fs::create_dir_all(project_path.join("src")).unwrap();
        fs::write(project_path.join("template-local.yaml"), "Resources:\nEvents:\n").unwrap();

        let gen = Generator::new(project_path.to_str().unwrap());

        let src_content = r#"
#[route("GET /test")]
pub async fn get_test() {}
"#;
        fs::write(project_path.join("src").join("handlers.rs"), src_content).unwrap();

        gen.run("routes");
        gen.run("openapi");
        gen.run("template");

        assert!(project_path.join("src").join("routes.rs").exists());

        let appid = project_path.file_name().unwrap().to_str().unwrap();
        let expected_schema = format!("{}-API-openapi-schema.json", appid);
        assert!(project_path.join(&expected_schema).exists());
    }
}
