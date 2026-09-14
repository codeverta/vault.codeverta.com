export const itemTypes = ["API Key", "Secret Key", "Access Token", "Password", "SSH Private Key", "Environment Variable", "Recovery Code", "License Key", "Secure Note", "Plain Text", "JSON", ".env", "Other"] as const;
export type ItemType = typeof itemTypes[number];

export interface VaultItem {
  id: string;
  title: string;
  item_type: ItemType;
  username: string;
  secret: string;
  notes: string;
  tags: string[];
  favorite: boolean;
  created_at: number;
  updated_at: number;
}

export type ItemDraft = Omit<VaultItem, "id" | "created_at" | "updated_at">;

export const emptyDraft = (): ItemDraft => ({
  title: "", item_type: "API Key", username: "", secret: "", notes: "", tags: [], favorite: false
});
