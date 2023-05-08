// @generated
impl serde::Serialize for CreateTransactionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
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
            serializer.serialize_struct("node.v1.CreateTransactionRequest", len)?;
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
impl<'de> serde::Deserialize<'de> for CreateTransactionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
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
            type Value = CreateTransactionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct node.v1.CreateTransactionRequest")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<CreateTransactionRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
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
                Ok(CreateTransactionRequest {
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
            "node.v1.CreateTransactionRequest",
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
        let struct_ser = serializer.serialize_struct("node.v1.GetFullMempoolRequest", len)?;
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
                formatter.write_str("struct node.v1.GetFullMempoolRequest")
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
        deserializer.deserialize_struct("node.v1.GetFullMempoolRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("node.v1.GetFullMempoolResponse", len)?;
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
                formatter.write_str("struct node.v1.GetFullMempoolResponse")
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
        deserializer.deserialize_struct("node.v1.GetFullMempoolResponse", FIELDS, GeneratedVisitor)
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
        let struct_ser = serializer.serialize_struct("node.v1.GetNodeTypeRequest", len)?;
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
                formatter.write_str("struct node.v1.GetNodeTypeRequest")
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
        deserializer.deserialize_struct("node.v1.GetNodeTypeRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("node.v1.GetNodeTypeResponse", len)?;
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
                formatter.write_str("struct node.v1.GetNodeTypeResponse")
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
        deserializer.deserialize_struct("node.v1.GetNodeTypeResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("node.v1.Token", len)?;
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
                formatter.write_str("struct node.v1.Token")
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
        deserializer.deserialize_struct("node.v1.Token", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("node.v1.TransactionRecord", len)?;
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
                formatter.write_str("struct node.v1.TransactionRecord")
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
        deserializer.deserialize_struct("node.v1.TransactionRecord", FIELDS, GeneratedVisitor)
    }
}
