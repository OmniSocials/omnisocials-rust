use serde_json::Value;

use crate::client::Client;
use crate::error::Error;
use crate::types::ListPinterestProductsParams;

/// `client.pinterest()`: the product Pins of the connected Pinterest
/// account, used to tag products on a Pin.
#[derive(Debug, Clone, Copy)]
pub struct Pinterest<'a> {
    pub(crate) client: &'a Client,
}

impl Pinterest<'_> {
    /// `GET /pinterest/products` - list the product Pins of the connected
    /// Pinterest account. Pass a result's `pin_id` in `product_tags` inside
    /// the `pinterest` options of a post to tag the product on the Pin (max
    /// 24 per Pin). Pinterest only accepts a product Pin that is public,
    /// belongs to the same account and links to a website that account
    /// claimed; products of other merchants cannot be tagged.
    ///
    /// The `"catalog"` source reads the Pinterest catalog (with `price`,
    /// `currency`, `availability`, `item_id`) and needs catalog access,
    /// which is given one time in the OmniSocials composer (Pinterest
    /// options, Add products, Connect catalog). The `"pins"` source reads
    /// the account's own Pins and works on every connection; one call scans
    /// up to 250 Pins, so `products` can be empty while `bookmark` is set
    /// (call again with the bookmark). Without `source` the API uses
    /// `"catalog"` when the connection has catalog access, else `"pins"`.
    ///
    /// The response is not the usual `data` envelope: `{"products":
    /// [{"pin_id", "title", "description", "link", "image_url", "price",
    /// "currency", "availability", "item_id"}], "bookmark", "source",
    /// "catalog_access"}` plus `product_groups` and `product_group_id` for
    /// the catalog source, or `{"error": {"code", "message"}}` without
    /// `products` when the list could not be read, both with HTTP 200.
    /// `code` is one of `pinterest_not_connected`,
    /// `pinterest_catalog_access_required` or `platform_error`. A bad
    /// `source` or `product_group_id` fails with [`Error::Validation`]
    /// (status 400).
    pub async fn list_products(&self, params: ListPinterestProductsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(source) = params.source {
            query.push(("source", source));
        }
        if let Some(product_group_id) = params.product_group_id {
            query.push(("product_group_id", product_group_id));
        }
        if let Some(bookmark) = params.bookmark {
            query.push(("bookmark", bookmark));
        }
        if let Some(page_size) = params.page_size {
            query.push(("page_size", page_size.to_string()));
        }
        self.client.get("/pinterest/products", query).await
    }

    /// `GET /pinterest/products/validate?id=` - check whether a Pin can be
    /// used in `product_tags` before creating the post. `id` is a Pin id or
    /// a Pin link (`https://www.pinterest.com/pin/<id>/`). The response is
    /// `{"valid", "pin_id", ...}`; `"unverified": true` means the check
    /// could not run and the publish step is the final check.
    pub async fn validate_product(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get("/pinterest/products/validate", vec![("id", id.to_owned())])
            .await
    }
}
