import { invoke } from "@tauri-apps/api/core";
import type { ItemDraft, VaultItem } from "./types";

export const api = {
  status: () => invoke<{ initialized: boolean; unlocked: boolean }>("vault_status"),
  create: (password: string) => invoke<void>("create_vault", { password }),
  unlock: (password: string) => invoke<void>("unlock_vault", { password }),
  lock: () => invoke<void>("lock_vault"),
  list: () => invoke<VaultItem[]>("list_items"),
  save: (item: ItemDraft & { id?: string }) => invoke<VaultItem>("save_item", { item }),
  remove: (id: string) => invoke<void>("delete_item", { id }),
  exportVault: (password: string, path: string) => invoke<string>("export_vault", { password, path }),
  importVault: (password: string, path: string) => invoke<number>("import_vault", { password, path }),
  changePassword: (currentPassword: string, newPassword: string) => invoke<void>("change_master_password", { currentPassword, newPassword })
};
