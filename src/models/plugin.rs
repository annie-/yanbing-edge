use std::env;
use std::ffi::OsStr;
use std::path::Path;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use validator::{Validate, ValidationError, ValidationErrors};
use std::env::consts::DLL_EXTENSION;
use crate::config::error::EdgeError;

// 公共的插件配置,创建使用
#[derive(Debug, Deserialize)]
pub struct CreatePluginConfig {
    pub description: Option<String>,
    pub form_customization: Option<String>,
    pub plugin: CreatePlugin,
    pub plugin_type: PluginType,
}

#[derive(Debug, Serialize)]
pub struct PluginConfig {
    pub id: i64,
    pub description: Option<String>,
    pub form_customization: Option<String>,
    pub plugin: Plugin,
    pub plugin_type: PluginType,
}

// 插件类型
#[derive(Debug, Serialize, Deserialize, Type)]
pub enum PluginType {
    // 系统插件
    #[serde(rename = "System")]
    System,
    // 自定义插件
    #[serde(rename = "Custom")]
    Custom,
}

// 插件类型枚举
#[derive(Debug, Serialize, Deserialize)]
pub enum Plugin {
    Protocol(ProtocolConfig),
    DataOutput(DataOutputConfig),
    RuleEngine(RuleEngineConfig),
}

// 插件类型枚举
#[derive(Debug, Deserialize)]
pub enum CreatePlugin {
    Protocol(CreateProtocolConfig),
    DataOutput(DataOutputConfig),
    RuleEngine(RuleEngineConfig),
}

// 南向协议解析插件配置
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ProtocolConfig {
    pub id: i64,
    //协议名称
    pub name: String,
    //协议库路径
    pub path: String,
    //协议描述
    pub description: Option<String>,
    //协议id
    pub plugin_config_id: i64,
}

// 南向协议解析插件配置
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProtocolConfig {
    //协议名称
    pub name: String,
    //协议库路径
    #[validate(custom = "validate_path")]
    pub path: String,
    //协议描述
    pub description: Option<String>,
}

fn validate_path(path: &str) -> Result<(), ValidationError> {

    let file_path = Path::new(path);
    // Check if the file exists
    if !file_path.exists() {
        return Err(ValidationError::new("库函数不存在"));
    }

    // Check the file extension
    let extension = file_path.extension().and_then(|ext| ext.to_str());
    match extension {
        Some(ext) if DLL_EXTENSION.eq(ext) => Ok(()),
        _ => Err(ValidationError::new("库函数不支持当前系统"))
    }
}

// 北向数据输出插件配置
#[derive(Debug, Serialize, Deserialize)]
pub struct DataOutputConfig {
    // 北向数据输出插件特有的字段
    // ...
}

// 规则引擎插件配置
#[derive(Debug, Serialize, Deserialize)]
pub struct RuleEngineConfig {
    // 规则引擎插件特有的字段
    // ...
}