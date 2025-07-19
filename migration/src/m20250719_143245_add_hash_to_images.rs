use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add hash column to images table for duplicate detection
        manager
            .alter_table(
                Table::alter()
                    .table(Images::Table)
                    .add_column(
                        ColumnDef::new(Images::Hash)
                            .string()
                            .null() // Allow null initially for existing records
                    )
                    .to_owned(),
            )
            .await?;

        // Add unique index on hash column to prevent duplicate uploads
        manager
            .create_index(
                Index::create()
                    .name("idx_images_hash_unique")
                    .table(Images::Table)
                    .col(Images::Hash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the unique index first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_images_hash_unique")
                    .table(Images::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the hash column
        manager
            .alter_table(
                Table::alter()
                    .table(Images::Table)
                    .drop_column(Images::Hash)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

// Define the Images table enum with the new Hash column
#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
    Filename,
    OriginalName,
    Size,
    MimeType,
    CreatedAt,
    Hash, // New column for image hash
}

// TODO: Define your table enum if needed
// #[derive(DeriveIden)]
// enum YourTable {
//     Table,
//     Id,
//     Name,
//     // Add more columns as needed
// }
