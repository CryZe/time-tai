use tai_stuff::TaiDateTime;
use time::{macros::datetime, OffsetDateTime};

fn main() {
    // let ts = OffsetDateTime::now_utc();
    let before = TaiDateTime::from(datetime!(1972-06-30 23:59:59 UTC));
    // let before = datetime!(1973-07-01 00:00:00 UTC);
    // dbg!(OffsetDateTime::from(TaiDateTime::from(before)));
    // dbg!(before.unix_timestamp());
    // dbg!(to_tai(before));

    // let after = datetime!(1972-07-01 00:00:00 UTC);
    let after = TaiDateTime::now();
    // dbg!(after.unix_timestamp());
    // dbg!(to_tai(after));

    dbg!(OffsetDateTime::from(after) - OffsetDateTime::from(before));
    dbg!(after - before);
}
