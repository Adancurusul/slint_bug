
pub mod SQLHandle{
    use rusqlite::{Connection, Result};
    #[derive(Debug)]
    pub struct MainExpData {
        pub  id: i32,
        pub  device_id: String,
        pub  exp_time: Option<i64>,
        pub  data_volume: Option<i64>,
        pub  is_deleted: Option<i64>,
    }
    #[derive(Debug)]
    pub struct TempratureData {
        pub  id: i64,
        pub  exp_id: Option<i32>,
        pub  record_time: Option<i64>,
        pub  data: Option<f32>,
        pub count: Option<i64>,
        pub  is_deleted: Option<i64>,
    }
    pub fn sql_create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            
            "create table if not exists Main (
                id    INTEGER PRIMARY KEY,
                deviceID  TEXT NOT NULL,
                expTime  INTEGER,
                dataVolume  INTEGER,
                isDeleted  INTEGER
            )",
            (), // empty list of parameters.
        )?;
        conn.execute(
            
            "create table if not exists Data (
                id    INTEGER PRIMARY KEY,
                expID  INTEGER,
                recordTime  INTEGER,
                data  REAL,
                isDeleted  INTEGER,
                count  INTEGER
            )",
            (), // empty list of parameters.
        )?;
        Ok(())
    }

    pub fn MAIN_sql_insert_main_table(conn: &Connection,data: &MainExpData) ->Result<()>{
        conn.execute(
            "INSERT INTO Main ( deviceID, expTime, dataVolume, isDeleted) VALUES (?, ?, ?, ?)",
            (&data.device_id, &data.exp_time, &data.data_volume, &data.is_deleted),
        )?;
        Ok(())
    }

    pub fn DATA_sql_insert_data_table(conn: &Connection,data: &TempratureData) ->Result<()>{
        conn.execute(
            "INSERT INTO Data ( expID, recordTime, data, isDeleted,count) VALUES (?, ?, ?, ?,?)",
            (&data.exp_id, &data.record_time, &data.data, &data.is_deleted,&data.count),
        )?;
        Ok(())
    }
    pub fn MAIN_get_data_by_device_id(//获取某次时间的所有实验
        conn: &Connection,
        device_id: &str,
    ) -> Result<Vec<MainExpData>> {
        let mut stmt = conn.prepare("SELECT * FROM Main WHERE deviceID = ?")?;
        let rows = stmt.query_map([device_id], |row| {
            Ok(MainExpData {
                id: row.get(0)?,
                device_id: row.get(1)?,
                exp_time: row.get(2)?,
                data_volume: row.get(3)?,
                is_deleted: row.get(4)?,
            })
        })?;

        let mut data_list = Vec::new();
        for row in rows {
            let data = row?;
            data_list.push(data);
        }

        Ok(data_list)
    }
    pub fn MAIN_modify_data_volume_by_id(
        conn: &Connection,
        id: i32,
        data_volume: i64,
    ) -> Result<()> {
        let mut stmt = conn.prepare("UPDATE Main SET dataVolume = ? WHERE id = ?")?;
        stmt.execute([&data_volume.to_string(), &id.to_string()])?;
        Ok(())
    }
    pub fn MAIN_get_data_by_exp_time(//获取某次时间的所有实验
        conn: &Connection,
        exp_time: i64,
    ) -> Result<Vec<MainExpData>> {
        let mut stmt = conn.prepare("SELECT * FROM Main WHERE expTime = ?")?;
        let rows = stmt.query_map([&exp_time.to_string()], |row| {
            Ok(MainExpData {
                id: row.get(0)?,
                device_id: row.get(1)?,
                exp_time: row.get(2)?,
                data_volume: row.get(3)?,
                is_deleted: row.get(4)?,
            })
        })?;

        let mut data_list = Vec::new();
        for row in rows {
            let data = row?;
            data_list.push(data);
        }

        Ok(data_list)
    }

    pub fn MAIN_get_data_by_device_id_and_exp_time(//获取某次实验特定传感器数据
        conn: &Connection,
        device_id: &str,
        exp_time: i64,
    ) -> Result<Vec<MainExpData>> {
        let mut stmt = conn.prepare("SELECT * FROM Main WHERE deviceID = ? AND expTime = ?")?;
        let rows = stmt.query_map([device_id, &exp_time.to_string()], |row| {
            Ok(MainExpData {
                id: row.get(0)?,
                device_id: row.get(1)?,
                exp_time: row.get(2)?,
                data_volume: row.get(3)?,
                is_deleted: row.get(4)?,
            })
        })?;

        let mut data_list = Vec::new();
        for row in rows {
            let data = row?;
            data_list.push(data);
        }

        Ok(data_list)
    }
    pub fn MAIN_update_is_deleted_by_exp_id(conn: &Connection, exp_id: i64) -> Result<()> {
        conn.execute(
            "UPDATE Main SET isDeleted = 1 WHERE expID = ?",
            &[&exp_id],
        )?;
        Ok(())
    }
    pub fn MAIN_delete_data_by_exp_id(conn: &Connection, exp_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM Main WHERE expID = ?",
            &[&exp_id],
        )?;
        Ok(())
    }
    pub fn DATA_update_is_deleted_by_exp_id(conn: &Connection, exp_id: i64) -> Result<()> {
        conn.execute(
            "UPDATE Data SET isDeleted = 1 WHERE expID = ?",
            &[&exp_id],
        )?;
        Ok(())
    }

    pub fn DATA_delete_data_by_exp_id(conn: &Connection, exp_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM Data WHERE expID = ?",
            &[&exp_id],
        )?;
        Ok(())
    }
    pub fn DATA_get_data_by_exp_id(conn: &Connection, exp_id: i64) -> Result<Vec<TempratureData>> {
        let mut stmt = conn.prepare("SELECT * FROM Data WHERE expID = ?")?;
        let rows = stmt.query_map([&exp_id.to_string()], |row| {
            Ok(TempratureData {
                id: row.get(0)?,
                exp_id: row.get(1)?,
                record_time: row.get(2)?,
                data: row.get(3)?,
                is_deleted: row.get(4)?,
                count: row.get(5)?,
            })
        })?;
    
        let mut data_list = Vec::new();
        for row in rows {
            let data = row?;
            data_list.push(data);
        }
    
        Ok(data_list)
    }



}