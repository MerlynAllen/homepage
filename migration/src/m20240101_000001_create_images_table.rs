use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建images表
        manager
            .create_table(
                Table::create()
                    .table(Images::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Images::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Images::Filename).string().not_null())
                    .col(ColumnDef::new(Images::OriginalName).string())
                    .col(ColumnDef::new(Images::Size).big_integer())
                    .col(ColumnDef::new(Images::MimeType).string())
                    .col(
                        ColumnDef::new(Images::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Images::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
    Filename,
    OriginalName,
    Size,
    MimeType,
    CreatedAt,
}
