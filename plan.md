# Support multiple shelf
1. Multiple shelf is allowing user to popup multiple shelf, it like a new environment for files selection
2. The logic of shelf already designed on app/shelf/module.rs
3. It currently only support one shelf, we need to support multiple shelf
4. Create an entities for a shelf (Created at, id: u64)
5. A LocalResource and LocalResourceId now need to including the shelf id
6. There is always a default shelf, with id = 0
7. A transfer session have type TransferType::Send, we convert it to TransferType::Send { from_shelf_id: u64 } to indicate the which shelf it is belong to
8. Web not support multiple shelf, only desktop was supported, so on web it always use default shelf
9. The shelf/commands.rs need to add function load_shelfs() which run at start, it also load all resources of each shelf