use crate::errs::{EngineError, EngineResult};
use base::codec::encode_varint64;
use meta::types::BqlType;
use mysql::{consts::ColumnType, prelude::FromValue, Column, FromValueError, Row, Value};
use mysql_common::{
    bigdecimal::BigDecimal,
    chrono::{Duration, NaiveDate, NaiveDateTime},
};

#[inline]
pub fn col_to_bql_type(c: &Column) -> EngineResult<BqlType> {
    let t = match c.column_type() {
        ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL => {
            BqlType::Decimal(
                8, // Maximum precision
                c.decimals(),
            )
        }
        ColumnType::MYSQL_TYPE_TINY => BqlType::Int(8),
        ColumnType::MYSQL_TYPE_SHORT => BqlType::Int(16),
        ColumnType::MYSQL_TYPE_LONG => BqlType::Int(32),
        ColumnType::MYSQL_TYPE_FLOAT => BqlType::Float(32),
        ColumnType::MYSQL_TYPE_DOUBLE => BqlType::Float(64),
        ColumnType::MYSQL_TYPE_NULL => {
            unimplemented!()
        }
        ColumnType::MYSQL_TYPE_TIMESTAMP | ColumnType::MYSQL_TYPE_TIMESTAMP2 => {
            BqlType::DateTime
        }

        ColumnType::MYSQL_TYPE_LONGLONG => BqlType::Int(64),
        ColumnType::MYSQL_TYPE_INT24 => BqlType::Int(32),
        ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_NEWDATE => BqlType::Date,
        ColumnType::MYSQL_TYPE_TIME | ColumnType::MYSQL_TYPE_TIME2 => BqlType::Int(64),
        ColumnType::MYSQL_TYPE_DATETIME | ColumnType::MYSQL_TYPE_DATETIME2 => {
            BqlType::DateTime
        }

        ColumnType::MYSQL_TYPE_YEAR => BqlType::UInt(16),
        ColumnType::MYSQL_TYPE_VARCHAR
        | ColumnType::MYSQL_TYPE_STRING
        | ColumnType::MYSQL_TYPE_VAR_STRING => BqlType::String,
        ColumnType::MYSQL_TYPE_BIT
        | ColumnType::MYSQL_TYPE_TYPED_ARRAY
        | ColumnType::MYSQL_TYPE_UNKNOWN
        | ColumnType::MYSQL_TYPE_JSON
        | ColumnType::MYSQL_TYPE_ENUM
        | ColumnType::MYSQL_TYPE_SET
        | ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB
        | ColumnType::MYSQL_TYPE_GEOMETRY => {
            unimplemented!()
        }
    };

    Ok(t)
}

pub fn get_val_bytes_from_row(
    row: &Row,
    offset_map: &mut Option<Vec<u32>>,
    null_map: &mut Vec<u8>,
    size: &mut usize,
) -> EngineResult<Vec<u8>> {
    let mut buf = vec![];
    for (i, c) in row.columns().as_ref().iter().enumerate() {
        match c.column_type() {
            ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL => {
                let n: Option<BigDecimal> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    let mut bs = n.into_bigint_and_exponent().0.to_signed_bytes_le();
                    if bs.len() < 4 {
                        bs.extend(vec![0; 4 - bs.len()]);
                    } else if bs.len() < 8 {
                        bs.extend(vec![0; 8 - bs.len()]);
                    } else {
                        return Err(EngineError::WrappingMySQLClientFromError(
                            FromValueError(row[i].clone()),
                        ));
                    }
                    buf.extend(bs);
                    null_map.push(0);
                } else {
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_TINY => {
                let n: Option<i8> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i8.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_SHORT => {
                let n: Option<i16> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i16.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_LONG => {
                let n: Option<i32> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i32.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_FLOAT => {
                let n: Option<f32> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0.0_f32.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_DOUBLE => {
                let n: Option<f64> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0.0_f64.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_NULL => {
                return Err(EngineError::WrappingMySQLClientFromError(FromValueError(
                    row[i].clone(),
                )))
            }
            ColumnType::MYSQL_TYPE_TIMESTAMP | ColumnType::MYSQL_TYPE_TIMESTAMP2 => {
                let date_time: Option<NaiveDateTime> = get_val_from_row(row, i)?;
                if let Some(date_time) = date_time {
                    buf.extend((date_time.timestamp() as i32).to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i32.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_LONGLONG => {
                let n: Option<i64> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i64.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_INT24 => {
                let n: Option<i32> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i32.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_NEWDATE => {
                let date: Option<NaiveDate> = get_val_from_row(row, i)?;
                if let Some(date) = date {
                    let gerg_date: NaiveDate = NaiveDate::from_ymd(1970, 1, 1);
                    buf.extend(
                        ((date - gerg_date).num_seconds() / (86400)).to_le_bytes(),
                    );
                    null_map.push(0);
                } else {
                    buf.extend(0_i64.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_TIME | ColumnType::MYSQL_TYPE_TIME2 => {
                let date_time: Option<Duration> = get_val_from_row(row, i)?;
                if let Some(date_time) = date_time {
                    buf.extend(date_time.num_seconds().to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i64.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_DATETIME | ColumnType::MYSQL_TYPE_DATETIME2 => {
                let date_time: Option<NaiveDateTime> = get_val_from_row(row, i)?;
                if let Some(date_time) = date_time {
                    buf.extend((date_time.timestamp() as i32).to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_i32.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_YEAR => {
                let n: Option<u16> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    buf.extend(n.to_le_bytes());
                    null_map.push(0);
                } else {
                    buf.extend(0_u16.to_le_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_VARCHAR
            | ColumnType::MYSQL_TYPE_STRING
            | ColumnType::MYSQL_TYPE_VAR_STRING => {
                let n: Option<String> = get_val_from_row(row, i)?;
                if let Some(n) = n {
                    if let Some(map) = offset_map {
                        map.push(map.len() as u32 + n.len() as u32);
                    } else {
                        *offset_map = Some(vec![0, n.len() as u32 + 1]);
                    }
                    *size += 1;
                    buf.reserve(10);
                    unsafe {
                        buf.set_len(buf.len() + 10);
                    }
                    let vl = encode_varint64(n.len() as u64, &mut buf);
                    unsafe {
                        buf.set_len(buf.len() - (10 - vl));
                    }
                    buf.extend(n.as_bytes());
                    null_map.push(0);
                } else {
                    buf.extend("\0".as_bytes());
                    null_map.push(1);
                }
            }
            ColumnType::MYSQL_TYPE_BIT
            | ColumnType::MYSQL_TYPE_TYPED_ARRAY
            | ColumnType::MYSQL_TYPE_UNKNOWN
            | ColumnType::MYSQL_TYPE_JSON
            | ColumnType::MYSQL_TYPE_ENUM
            | ColumnType::MYSQL_TYPE_SET
            | ColumnType::MYSQL_TYPE_TINY_BLOB
            | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
            | ColumnType::MYSQL_TYPE_LONG_BLOB
            | ColumnType::MYSQL_TYPE_BLOB
            | ColumnType::MYSQL_TYPE_GEOMETRY => {
                return Err(EngineError::WrappingMySQLClientFromError(FromValueError(
                    row[i].clone(),
                )))
            }
        };
    }

    Ok(buf)
}

#[inline]
fn get_val_from_row<T>(row: &Row, i: usize) -> EngineResult<Option<T>>
where
    T: FromValue,
{
    let v: Option<Result<T, FromValueError>> = row.get_opt(i);

    match v {
        Some(Ok(n)) => Ok(Some(n)),
        Some(Err(FromValueError(Value::NULL))) => Ok(None),
        _ => Err(EngineError::WrappingMySQLClientFromError(FromValueError(
            row[i].clone(),
        ))),
    }
}
