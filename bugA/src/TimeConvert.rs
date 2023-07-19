
pub mod TimeConvert{
    use chrono::prelude::*;
    pub fn get_timestamp()->i64{
        let now: DateTime<Local> = Local::now();
        now.timestamp()
    }
    
    pub fn timestamp_to_str(timeStamp:i64) -> String{
        let fmt = "%Y-%m-%d-%H:%M:%S";
        let date_time = NaiveDateTime::from_timestamp(timeStamp, 0);
        //println!("{:?}", Local.);
        let date_time_local: DateTime<Local> = Local.from_utc_datetime(&date_time);
        let dtf = date_time_local.format(fmt);
        dtf.to_string()
    }
    pub fn str_to_timestamp(timeStr:String) -> i64{
        let fmt = "%Y-%m-%d-%H:%M:%S";
        let date_time = NaiveDateTime::parse_from_str(&timeStr, fmt).unwrap();
        let date_time_local: DateTime<Local> = Local.from_local_datetime(&date_time).unwrap();
        date_time_local.timestamp()
    }

}


