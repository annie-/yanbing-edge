use std::collections::HashMap;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use sqlx::SqlitePool;
use protocol_core::{Device, Point, PointWithProtocolId, Value, WriterPointRequest};
use crate::config::cache::get_protocol_store;
use crate::config::error::{EdgeError, Result};
use crate::models::device::{CreatDevice, DeviceDTO};
use crate::models::R;
use crate::config::device_shadow;


pub async fn get_device(State(pool): State<SqlitePool>, Path(id): Path<i32>) -> Result<Json<DeviceDTO>> {
    let device = sqlx::query_as::<_, DeviceDTO>("SELECT * FROM tb_device WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await?;
    match device {
        Some(device) => Ok(Json(device)),
        None => {
            // 没有找到匹配的行，返回自定义错误或其他逻辑
            Err(EdgeError::Message("设备不存在".into()))
        }
    }
}

pub async fn get_device_details(State(pool): State<SqlitePool>, Path(id): Path<i32>) -> Result<Json<Device>> {
    let device = sqlx::query_as::<_, DeviceDTO>("SELECT * FROM tb_device WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await?;

    let points = sqlx::query_as::<_, Point>("SELECT * FROM tb_point WHERE device_id = ?")
        .bind(device.id)
        .fetch_all(&pool)
        .await?;

    let device_with_points = Device {
        id: device.id,
        name: device.name,
        device_type: device.device_type,
        points,
        custom_data: device.custom_data.0,
        protocol_name: device.protocol_name,
    };

    Ok(Json(device_with_points))
}

pub async fn read_point_value(State(pool): State<SqlitePool>, Path(id): Path<i32>) -> Result<Json<Value>> {
    let point = get_point_with_protocol_id(pool, id).await?;
    let res = device_shadow::read_point(point.protocol_name.clone(), point.into())
        .map(|e| e.value)?;
    Ok(Json(res))
}

#[derive(Debug, Deserialize)]
pub struct WriterValue{
    value:Value,
}

pub async fn writer_point_value(State(pool): State<SqlitePool>,
                                Path(id): Path<i32>,
                                Json(WriterValue{value, .. }): Json<WriterValue>) -> Result<Json<Value>> {
    let point = get_point_with_protocol_id(pool, id).await?;
    let store = get_protocol_store().unwrap();
    let protocol_map = store.inner.read().unwrap();
    let protocol = protocol_map.get(&point.protocol_name).ok_or(EdgeError::Message("协议不存在,检查服务配置".into()))?;
    let mut request: WriterPointRequest = point.into();
    request.value = value;
    let res = protocol.read().unwrap()
        .write_point(request)?;
    Ok(Json(res))
}

async fn get_point_with_protocol_id(pool: SqlitePool, id: i32) -> Result<PointWithProtocolId> {
    let point = sqlx::query_as::<_, PointWithProtocolId>(r#"
    SELECT tb_point.id AS point_id, tb_point.device_id, tb_point.address, tb_point.data_type, tb_point.access_mode,
       tb_point.multiplier, tb_point.precision, tb_point.description, tb_point.part_number, tb_device.protocol_name AS protocol_name
        FROM tb_point
        JOIN tb_device ON tb_point.device_id = tb_device.id
        WHERE tb_point.id = ?;
    "#)
        .bind(id)
        .fetch_optional(&pool)
        .await?;
    let point = match point {
        Some(point) => point,
        None => {
            return Err(EdgeError::Message("point不存在,请检查请求参数".into()));
        }
    };
    Ok(point)
}


pub async fn load_all_device_details(pool: SqlitePool) -> Result<HashMap<String, Vec<Device>>> {
    let device_list = sqlx::query_as::<_, DeviceDTO>("SELECT * FROM tb_device")
        .fetch_all(&pool)
        .await?;
    let mut res: HashMap<String, Vec<Device>> = HashMap::new();
    for device in device_list.iter() {
        let points = sqlx::query_as::<_, Point>("SELECT * FROM tb_point WHERE device_id = ?")
            .bind(device.id)
            .fetch_all(&pool)
            .await?;
        let device_with_points = Device {
            id: device.id,
            name: device.name.clone(),
            device_type: device.device_type.clone(),
            points,
            custom_data: device.custom_data.0.clone(),
            protocol_name: device.protocol_name.clone(),
        };
        // 插入方式简洁处理
        res.entry(device.protocol_name.clone())
            .or_insert_with(Vec::new)
            .push(device_with_points);
    }
    tracing::info!("加载协议总数:{}",res.len());
    Ok(res)
}

pub async fn create_device(State(pool): State<SqlitePool>, device: Json<CreatDevice>) -> Result<Json<R<DeviceDTO>>> {
    let created_device = sqlx::query_as::<_, DeviceDTO>(
        "INSERT INTO tb_device (name, device_type, custom_data, protocol_name) VALUES (?, ?, ?, ?) RETURNING *",
    )
        .bind(&device.name)
        .bind(&device.device_type)
        .bind(sqlx::types::Json(&device.custom_data))
        .bind(device.protocol_name.clone())
        .fetch_one(&pool)
        .await?;

    Ok(Json(R::success_with_data(created_device)))
}

pub async fn update_device(
    State(pool): State<SqlitePool>,
    Path(id): Path<i32>,
    Json(device): Json<DeviceDTO>,
) -> Result<Json<R<String>>> {
    let updated_device = sqlx::query(
        "UPDATE tb_device SET name = $1, device_type = $2, custom_data = $3, protocol_name = $4 WHERE id = $5",
    )
        .bind(&device.name)
        .bind(&device.device_type)
        .bind(sqlx::types::Json(&device.custom_data))
        .bind(device.protocol_name)
        .bind(id)
        .execute(&pool)
        .await?;

    // 检查是否成功更新了设备
    if updated_device.rows_affected() > 0 {
        // 返回更新后的设备
        Ok(Json(R::success()))
    } else {
        // 如果没有更新设备，则返回错误信息
        Err(EdgeError::Message("设备不存在".into()))
    }
}

pub async fn delete_device(State(pool): State<SqlitePool>, Path(device_id): Path<i32>) -> Result<Json<R<String>>> {
    sqlx::query("DELETE FROM tb_device WHERE id = ?")
        .bind(device_id)
        .execute(&pool)
        .await?;

    Ok(Json(R::success()))
}
