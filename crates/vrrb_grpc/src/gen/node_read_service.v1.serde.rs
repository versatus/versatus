// @generated
impl serde::Serialize for Account {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.account_nonce != 0 {
            len += 1;
        }
        if self.credits != 0 {
            len += 1;
        }
        if self.debits != 0 {
            len += 1;
        }
        if !self.storage.is_empty() {
            len += 1;
        }
        if !self.code.is_empty() {
            len += 1;
        }
        if !self.pubkey.is_empty() {
            len += 1;
        }
        if self.digests.is_some() {
            len += 1;
        }
        if self.created_at != 0 {
            len += 1;
        }
        if self.updated_at != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("node_read_service.v1.Account", len)?;
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if self.account_nonce != 0 {
            struct_ser.serialize_field(
                "accountNonce",
                ToString::to_string(&self.account_nonce).as_str(),
            )?;
        }
        if self.credits != 0 {
            struct_ser.serialize_field("credits", ToString::to_string(&self.credits).as_str())?;
        }
        if self.debits != 0 {
            struct_ser.serialize_field("debits", ToString::to_string(&self.debits).as_str())?;
        }
        if !self.storage.is_empty() {
            struct_ser.serialize_field("storage", &self.storage)?;
        }
        if !self.code.is_empty() {
            struct_ser.serialize_field("code", &self.code)?;
        }
        if !self.pubkey.is_empty() {
            struct_ser.serialize_field("pubkey", &self.pubkey)?;
        }
        if let Some(v) = self.digests.as_ref() {
            struct_ser.serialize_field("digests", v)?;
        }
        if self.created_at != 0 {
            struct_ser
                .serialize_field("createdAt", ToString::to_string(&self.created_at).as_str())?;
        }
        if self.updated_at != 0 {
            struct_ser
                .serialize_field("updatedAt", ToString::to_string(&self.updated_at).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Account {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "hash",
            "account_nonce",
            "accountNonce",
            "credits",
            "debits",
            "storage",
            "code",
            "pubkey",
            "digests",
            "created_at",
            "createdAt",
            "updated_at",
            "updatedAt",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Hash,
            AccountNonce,
            Credits,
            Debits,
            Storage,
            Code,
            Pubkey,
            Digests,
            CreatedAt,
            UpdatedAt,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "hash" => Ok(GeneratedField::Hash),
                            "accountNonce" | "account_nonce" => Ok(GeneratedField::AccountNonce),
                            "credits" => Ok(GeneratedField::Credits),
                            "debits" => Ok(GeneratedField::Debits),
                            "storage" => Ok(GeneratedField::Storage),
                            "code" => Ok(GeneratedField::Code),
                            "pubkey" => Ok(GeneratedField::Pubkey),
                            "digests" => Ok(GeneratedField::Digests),
                            "createdAt" | "created_at" => Ok(GeneratedField::CreatedAt),
                            "updatedAt" | "updated_at" => Ok(GeneratedField::UpdatedAt),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Account;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.Account")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Account, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut hash__ = None;
                let mut account_nonce__ = None;
                let mut credits__ = None;
                let mut debits__ = None;
                let mut storage__ = None;
                let mut code__ = None;
                let mut pubkey__ = None;
                let mut digests__ = None;
                let mut created_at__ = None;
                let mut updated_at__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map.next_value()?);
                        },
                        GeneratedField::AccountNonce => {
                            if account_nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accountNonce"));
                            }
                            account_nonce__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                        GeneratedField::Credits => {
                            if credits__.is_some() {
                                return Err(serde::de::Error::duplicate_field("credits"));
                            }
                            credits__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                        GeneratedField::Debits => {
                            if debits__.is_some() {
                                return Err(serde::de::Error::duplicate_field("debits"));
                            }
                            debits__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                        GeneratedField::Storage => {
                            if storage__.is_some() {
                                return Err(serde::de::Error::duplicate_field("storage"));
                            }
                            storage__ = Some(map.next_value()?);
                        },
                        GeneratedField::Code => {
                            if code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code__ = Some(map.next_value()?);
                        },
                        GeneratedField::Pubkey => {
                            if pubkey__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pubkey"));
                            }
                            pubkey__ = Some(map.next_value()?);
                        },
                        GeneratedField::Digests => {
                            if digests__.is_some() {
                                return Err(serde::de::Error::duplicate_field("digests"));
                            }
                            digests__ = map.next_value()?;
                        },
                        GeneratedField::CreatedAt => {
                            if created_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createdAt"));
                            }
                            created_at__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                        GeneratedField::UpdatedAt => {
                            if updated_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updatedAt"));
                            }
                            updated_at__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                    }
                }
                Ok(Account {
                    hash: hash__.unwrap_or_default(),
                    account_nonce: account_nonce__.unwrap_or_default(),
                    credits: credits__.unwrap_or_default(),
                    debits: debits__.unwrap_or_default(),
                    storage: storage__.unwrap_or_default(),
                    code: code__.unwrap_or_default(),
                    pubkey: pubkey__.unwrap_or_default(),
                    digests: digests__,
                    created_at: created_at__.unwrap_or_default(),
                    updated_at: updated_at__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("node_read_service.v1.Account", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AccountDigests {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.sent.is_some() {
            len += 1;
        }
        if self.recv.is_some() {
            len += 1;
        }
        if self.stake.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.AccountDigests", len)?;
        if let Some(v) = self.sent.as_ref() {
            struct_ser.serialize_field("sent", v)?;
        }
        if let Some(v) = self.recv.as_ref() {
            struct_ser.serialize_field("recv", v)?;
        }
        if let Some(v) = self.stake.as_ref() {
            struct_ser.serialize_field("stake", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AccountDigests {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["sent", "recv", "stake"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Sent,
            Recv,
            Stake,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sent" => Ok(GeneratedField::Sent),
                            "recv" => Ok(GeneratedField::Recv),
                            "stake" => Ok(GeneratedField::Stake),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AccountDigests;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.AccountDigests")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<AccountDigests, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut sent__ = None;
                let mut recv__ = None;
                let mut stake__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Sent => {
                            if sent__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sent"));
                            }
                            sent__ = map.next_value()?;
                        },
                        GeneratedField::Recv => {
                            if recv__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recv"));
                            }
                            recv__ = map.next_value()?;
                        },
                        GeneratedField::Stake => {
                            if stake__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stake"));
                            }
                            stake__ = map.next_value()?;
                        },
                    }
                }
                Ok(AccountDigests {
                    sent: sent__,
                    recv: recv__,
                    stake: stake__,
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.AccountDigests",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for FullStateSnapshotRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser =
            serializer.serialize_struct("node_read_service.v1.FullStateSnapshotRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FullStateSnapshotRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {}
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FullStateSnapshotRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.FullStateSnapshotRequest")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<FullStateSnapshotRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(FullStateSnapshotRequest {})
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.FullStateSnapshotRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for FullStateSnapshotResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.full_state_snapshot.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.FullStateSnapshotResponse", len)?;
        if !self.full_state_snapshot.is_empty() {
            struct_ser.serialize_field("fullStateSnapshot", &self.full_state_snapshot)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FullStateSnapshotResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["full_state_snapshot", "fullStateSnapshot"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FullStateSnapshot,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "fullStateSnapshot" | "full_state_snapshot" => {
                                Ok(GeneratedField::FullStateSnapshot)
                            },
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FullStateSnapshotResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.FullStateSnapshotResponse")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<FullStateSnapshotResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut full_state_snapshot__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::FullStateSnapshot => {
                            if full_state_snapshot__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fullStateSnapshot"));
                            }
                            full_state_snapshot__ =
                                Some(map.next_value::<std::collections::HashMap<_, _>>()?);
                        },
                    }
                }
                Ok(FullStateSnapshotResponse {
                    full_state_snapshot: full_state_snapshot__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.FullStateSnapshotResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetAccountRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetAccountRequest", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetAccountRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["address"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetAccountRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetAccountRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetAccountRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        },
                    }
                }
                Ok(GetAccountRequest {
                    address: address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetAccountRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetAccountResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.account.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetAccountResponse", len)?;
        if let Some(v) = self.account.as_ref() {
            struct_ser.serialize_field("account", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetAccountResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["account"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Account,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "account" => Ok(GeneratedField::Account),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetAccountResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetAccountResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetAccountResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut account__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Account => {
                            if account__.is_some() {
                                return Err(serde::de::Error::duplicate_field("account"));
                            }
                            account__ = map.next_value()?;
                        },
                    }
                }
                Ok(GetAccountResponse { account: account__ })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetAccountResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetFullMempoolRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetFullMempoolRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetFullMempoolRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {}
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetFullMempoolRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetFullMempoolRequest")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<GetFullMempoolRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetFullMempoolRequest {})
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetFullMempoolRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetFullMempoolResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.transaction_records.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetFullMempoolResponse", len)?;
        if !self.transaction_records.is_empty() {
            struct_ser.serialize_field("transactionRecords", &self.transaction_records)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetFullMempoolResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["transaction_records", "transactionRecords"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionRecords,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "transactionRecords" | "transaction_records" => {
                                Ok(GeneratedField::TransactionRecords)
                            },
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetFullMempoolResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetFullMempoolResponse")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<GetFullMempoolResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut transaction_records__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TransactionRecords => {
                            if transaction_records__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "transactionRecords",
                                ));
                            }
                            transaction_records__ = Some(map.next_value()?);
                        },
                    }
                }
                Ok(GetFullMempoolResponse {
                    transaction_records: transaction_records__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetFullMempoolResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetNodeTypeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetNodeTypeRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeTypeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {}
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetNodeTypeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetNodeTypeRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetNodeTypeRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetNodeTypeRequest {})
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetNodeTypeRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetNodeTypeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.result.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetNodeTypeResponse", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if !self.result.is_empty() {
            struct_ser.serialize_field("result", &self.result)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeTypeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["id", "result"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Result,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "result" => Ok(GeneratedField::Result),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetNodeTypeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetNodeTypeResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetNodeTypeResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut result__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map.next_value()?);
                        },
                        GeneratedField::Result => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result__ = Some(map.next_value()?);
                        },
                    }
                }
                Ok(GetNodeTypeResponse {
                    id: id__.unwrap_or_default(),
                    result: result__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetNodeTypeResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetTransactionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rpc_transaction_digest.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetTransactionRequest", len)?;
        if !self.rpc_transaction_digest.is_empty() {
            struct_ser.serialize_field("rpcTransactionDigest", &self.rpc_transaction_digest)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["rpc_transaction_digest", "rpcTransactionDigest"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RpcTransactionDigest,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "rpcTransactionDigest" | "rpc_transaction_digest" => {
                                Ok(GeneratedField::RpcTransactionDigest)
                            },
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetTransactionRequest")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<GetTransactionRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut rpc_transaction_digest__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::RpcTransactionDigest => {
                            if rpc_transaction_digest__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "rpcTransactionDigest",
                                ));
                            }
                            rpc_transaction_digest__ = Some(map.next_value()?);
                        },
                    }
                }
                Ok(GetTransactionRequest {
                    rpc_transaction_digest: rpc_transaction_digest__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetTransactionRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GetTransactionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction_record.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.GetTransactionResponse", len)?;
        if let Some(v) = self.transaction_record.as_ref() {
            struct_ser.serialize_field("transactionRecord", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["transaction_record", "transactionRecord"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionRecord,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "transactionRecord" | "transaction_record" => {
                                Ok(GeneratedField::TransactionRecord)
                            },
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.GetTransactionResponse")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<GetTransactionResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut transaction_record__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TransactionRecord => {
                            if transaction_record__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionRecord"));
                            }
                            transaction_record__ = map.next_value()?;
                        },
                    }
                }
                Ok(GetTransactionResponse {
                    transaction_record: transaction_record__,
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.GetTransactionResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for ListTransactionsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.digests.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.ListTransactionsRequest", len)?;
        if !self.digests.is_empty() {
            struct_ser.serialize_field("digests", &self.digests)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTransactionsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["digests"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Digests,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "digests" => Ok(GeneratedField::Digests),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTransactionsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.ListTransactionsRequest")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<ListTransactionsRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut digests__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Digests => {
                            if digests__.is_some() {
                                return Err(serde::de::Error::duplicate_field("digests"));
                            }
                            digests__ = Some(map.next_value()?);
                        },
                    }
                }
                Ok(ListTransactionsRequest {
                    digests: digests__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.ListTransactionsRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for ListTransactionsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.transactions.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.ListTransactionsResponse", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTransactionsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["transactions"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transactions,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "transactions" => Ok(GeneratedField::Transactions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTransactionsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.ListTransactionsResponse")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<ListTransactionsResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut transactions__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ =
                                Some(map.next_value::<std::collections::HashMap<_, _>>()?);
                        },
                    }
                }
                Ok(ListTransactionsResponse {
                    transactions: transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.ListTransactionsResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for Token {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.symbol.is_empty() {
            len += 1;
        }
        if self.decimals != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("node_read_service.v1.Token", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.symbol.is_empty() {
            struct_ser.serialize_field("symbol", &self.symbol)?;
        }
        if self.decimals != 0 {
            struct_ser.serialize_field("decimals", &self.decimals)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Token {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["name", "symbol", "decimals"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Symbol,
            Decimals,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "symbol" => Ok(GeneratedField::Symbol),
                            "decimals" => Ok(GeneratedField::Decimals),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Token;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.Token")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Token, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut symbol__ = None;
                let mut decimals__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        },
                        GeneratedField::Symbol => {
                            if symbol__.is_some() {
                                return Err(serde::de::Error::duplicate_field("symbol"));
                            }
                            symbol__ = Some(map.next_value()?);
                        },
                        GeneratedField::Decimals => {
                            if decimals__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decimals"));
                            }
                            decimals__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                    }
                }
                Ok(Token {
                    name: name__.unwrap_or_default(),
                    symbol: symbol__.unwrap_or_default(),
                    decimals: decimals__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("node_read_service.v1.Token", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionDigest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.inner.is_empty() {
            len += 1;
        }
        if !self.digest_string.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.TransactionDigest", len)?;
        if !self.inner.is_empty() {
            struct_ser.serialize_field("inner", &self.inner)?;
        }
        if !self.digest_string.is_empty() {
            struct_ser.serialize_field("digestString", &self.digest_string)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionDigest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["inner", "digest_string", "digestString"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Inner,
            DigestString,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "inner" => Ok(GeneratedField::Inner),
                            "digestString" | "digest_string" => Ok(GeneratedField::DigestString),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionDigest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.TransactionDigest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionDigest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut inner__ = None;
                let mut digest_string__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Inner => {
                            if inner__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inner"));
                            }
                            inner__ = Some(map.next_value()?);
                        },
                        GeneratedField::DigestString => {
                            if digest_string__.is_some() {
                                return Err(serde::de::Error::duplicate_field("digestString"));
                            }
                            digest_string__ = Some(map.next_value()?);
                        },
                    }
                }
                Ok(TransactionDigest {
                    inner: inner__.unwrap_or_default(),
                    digest_string: digest_string__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.TransactionDigest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for TransactionRecord {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if self.timestamp != 0 {
            len += 1;
        }
        if !self.sender_address.is_empty() {
            len += 1;
        }
        if !self.sender_public_key.is_empty() {
            len += 1;
        }
        if !self.receiver_address.is_empty() {
            len += 1;
        }
        if self.token.is_some() {
            len += 1;
        }
        if self.amount != 0 {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        if !self.validators.is_empty() {
            len += 1;
        }
        if self.nonce != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("node_read_service.v1.TransactionRecord", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if self.timestamp != 0 {
            struct_ser
                .serialize_field("timestamp", ToString::to_string(&self.timestamp).as_str())?;
        }
        if !self.sender_address.is_empty() {
            struct_ser.serialize_field("senderAddress", &self.sender_address)?;
        }
        if !self.sender_public_key.is_empty() {
            struct_ser.serialize_field("senderPublicKey", &self.sender_public_key)?;
        }
        if !self.receiver_address.is_empty() {
            struct_ser.serialize_field("receiverAddress", &self.receiver_address)?;
        }
        if let Some(v) = self.token.as_ref() {
            struct_ser.serialize_field("token", v)?;
        }
        if self.amount != 0 {
            struct_ser.serialize_field("amount", ToString::to_string(&self.amount).as_str())?;
        }
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", &self.signature)?;
        }
        if !self.validators.is_empty() {
            struct_ser.serialize_field("validators", &self.validators)?;
        }
        if self.nonce != 0 {
            struct_ser.serialize_field("nonce", ToString::to_string(&self.nonce).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionRecord {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "timestamp",
            "sender_address",
            "senderAddress",
            "sender_public_key",
            "senderPublicKey",
            "receiver_address",
            "receiverAddress",
            "token",
            "amount",
            "signature",
            "validators",
            "nonce",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Timestamp,
            SenderAddress,
            SenderPublicKey,
            ReceiverAddress,
            Token,
            Amount,
            Signature,
            Validators,
            Nonce,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "senderAddress" | "sender_address" => Ok(GeneratedField::SenderAddress),
                            "senderPublicKey" | "sender_public_key" => {
                                Ok(GeneratedField::SenderPublicKey)
                            },
                            "receiverAddress" | "receiver_address" => {
                                Ok(GeneratedField::ReceiverAddress)
                            },
                            "token" => Ok(GeneratedField::Token),
                            "amount" => Ok(GeneratedField::Amount),
                            "signature" => Ok(GeneratedField::Signature),
                            "validators" => Ok(GeneratedField::Validators),
                            "nonce" => Ok(GeneratedField::Nonce),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node_read_service.v1.TransactionRecord")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionRecord, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut timestamp__ = None;
                let mut sender_address__ = None;
                let mut sender_public_key__ = None;
                let mut receiver_address__ = None;
                let mut token__ = None;
                let mut amount__ = None;
                let mut signature__ = None;
                let mut validators__ = None;
                let mut nonce__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map.next_value()?);
                        },
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                        GeneratedField::SenderAddress => {
                            if sender_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("senderAddress"));
                            }
                            sender_address__ = Some(map.next_value()?);
                        },
                        GeneratedField::SenderPublicKey => {
                            if sender_public_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("senderPublicKey"));
                            }
                            sender_public_key__ = Some(map.next_value()?);
                        },
                        GeneratedField::ReceiverAddress => {
                            if receiver_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("receiverAddress"));
                            }
                            receiver_address__ = Some(map.next_value()?);
                        },
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = map.next_value()?;
                        },
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = Some(map.next_value()?);
                        },
                        GeneratedField::Validators => {
                            if validators__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validators"));
                            }
                            validators__ =
                                Some(map.next_value::<std::collections::HashMap<_, _>>()?);
                        },
                        GeneratedField::Nonce => {
                            if nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonce"));
                            }
                            nonce__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        },
                    }
                }
                Ok(TransactionRecord {
                    id: id__.unwrap_or_default(),
                    timestamp: timestamp__.unwrap_or_default(),
                    sender_address: sender_address__.unwrap_or_default(),
                    sender_public_key: sender_public_key__.unwrap_or_default(),
                    receiver_address: receiver_address__.unwrap_or_default(),
                    token: token__,
                    amount: amount__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                    validators: validators__.unwrap_or_default(),
                    nonce: nonce__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "node_read_service.v1.TransactionRecord",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
