# Migration Tool

这是一个专门用于管理数据库迁移的工具，基于 Sea-ORM migration。

## 使用方法

在项目根目录下，您可以使用以下命令：

### 应用所有待处理的迁移
```bash
cargo migrate up
```

### 回滚最后一个迁移
```bash
cargo migrate down
```

### 检查迁移状态
```bash
cargo migrate status
```

### 生成新的迁移文件（提供文件名建议）
```bash
cargo migrate generate "迁移名称"
```

## 配置

默认数据库连接：`sqlite:./server/info.db`

您可以通过设置环境变量 `DATABASE_URL` 来指定不同的数据库连接：
```bash
export DATABASE_URL="sqlite:./custom/path/database.db"
cargo migrate up
```

## 添加新的迁移

1. 在 `migration/src/` 目录下创建新的迁移文件，例如：`m20250719_142238_create_users_table.rs`

2. 实现迁移逻辑（参考 `m20240101_000001_create_images_table.rs` 示例）

3. 在 `migration/src/lib.rs` 中的 `migrations()` 函数中添加您的新迁移：
```rust
fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20240101_000001_create_images_table::Migration),
        Box::new(m20250719_142238_create_users_table::Migration), // 添加新迁移
    ]
}
```

## 项目结构

```
migration/
├── src/
│   ├── lib.rs                                    # 主要的 Migrator 实现
│   ├── main.rs                                   # CLI 工具入口点
│   └── m20240101_000001_create_images_table.rs   # 示例迁移文件
├── Cargo.toml
└── README.md
```

## 示例迁移

参考 `m20240101_000001_create_images_table.rs` 来了解如何编写迁移：

- `up()` 方法：应用迁移（创建表、添加列等）
- `down()` 方法：回滚迁移（删除表、删除列等）
