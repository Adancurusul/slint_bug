# BUG A 

Unable to update data ```export global CT_mainPageAdapter```  from ```import {CT_mainPageAdapter} from "ui/CT_main_page.slint";```
in ```mian.rs [212]:         .set_historyPreviewData(show_His_model_now.clone().into());``` The table data was updated, but the references in ``CT_table_page.slint`` were not modified.

and  ```export component PlotArea inherits Image```  has similar bug.


![ErrorMain](./bugA/image/ErrorMain.png)
![ErrorTable](./bugA/image/ErrorTable.png)



