use std::net::IpAddr;

use diesel::backend::Backend;
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Binary;

#[derive(Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = Binary)]
pub struct IpAddress(IpAddr);

impl Into<IpAddr> for IpAddress {
    fn into(self) -> IpAddr {
        self.0
    }
}

// impl<DB> Queryable<Binary, DB> for IpAddress
// where
//     DB: Backend,
//     Vec<u8>: FromSql<Binary, DB>,
// {
//     type Row = Vec<u8>;

//     fn build(bytes: Self::Row) -> deserialize::Result<Self> {
//         match bytes.len() {
//             4 => Ok(IpAddress(IpAddr::from([
//                 bytes[0], bytes[1], bytes[2], bytes[3],
//             ]))),
//             16 => Ok(IpAddress(IpAddr::from(
//                 <[u8; 16]>::try_from(&bytes[0..16]).map_err(|_| "Invalid IPv6 address.")?,
//             ))),
//             _ => {
//                 let error: Box<dyn std::error::Error + Send + Sync> =
//                     Box::new(crate::error::DecodeError::IpAddress);
//                 Err(error)
//             }
//         }
//     }
// }

impl Into<IpAddress> for IpAddr {
    fn into(self) -> IpAddress {
        IpAddress(self)
    }
}

impl ToSql<Binary, diesel::mysql::Mysql> for IpAddress
where
    [u8; 4]: ToSql<Binary, diesel::mysql::Mysql>,
    [u8; 16]: ToSql<Binary, diesel::mysql::Mysql>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::mysql::Mysql>) -> serialize::Result {
        match self.0 {
            IpAddr::V4(ip) => <[u8; 4] as ToSql<Binary, diesel::mysql::Mysql>>::to_sql(
                &ip.octets(),
                &mut out.reborrow(),
            ),
            IpAddr::V6(ip) => <[u8; 16] as ToSql<Binary, diesel::mysql::Mysql>>::to_sql(
                &ip.octets(),
                &mut out.reborrow(),
            ),
        }
    }
}

impl<DB> FromSql<Binary, DB> for IpAddress
where
    DB: Backend,
    Vec<u8>: FromSql<Binary, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        // Ok(Self::try_from(<Vec<u8>>::from_sql(bytes)?)?)
        let bytes = <Vec<u8>>::from_sql(bytes)?;
        match bytes.len() {
            4 => Ok(IpAddress(IpAddr::from([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ]))),
            16 => Ok(IpAddress(IpAddr::from([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]))),
            _ => {
                let error: Box<dyn std::error::Error + Send + Sync> =
                    Box::new(crate::error::DecodeError::Ip);
                Err(error)
            }
        }
    }
}
