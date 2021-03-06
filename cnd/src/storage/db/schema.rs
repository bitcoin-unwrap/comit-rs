// LocalSwapId and SharedSwapId are encoded as Text and named local_swap_id, and
// shared_swap_id respectively.  swap_id (Integer) is always a foreign key link
// to the `swaps` table.
table! {
   swaps {
       id -> Integer,
       local_swap_id -> Text,
       role -> Text,
       counterparty_peer_id -> Text,
       start_of_swap -> BigInt,
   }
}

table! {
   secret_hashes {
       id -> Integer,
       swap_id -> Integer,
       secret_hash -> Text,
   }
}

table! {
    herc20s {
        id -> Integer,
        swap_id -> Integer,
        amount -> Text,
        chain_id -> BigInt,
        expiry -> BigInt,
        token_contract -> Text,
        redeem_identity -> Text,
        refund_identity -> Text,
        side -> Text,
    }
}

table! {
    hbits {
        id -> Integer,
        swap_id -> Integer,
        amount -> Text,
        network -> Text,
        expiry -> BigInt,
        final_identity -> Text,
        transient_identity -> Text,
        side -> Text,
    }
}

table! {
    swap_contexts {
        id -> Text,
        role -> Text,
        alpha -> Text,
        beta -> Text,
    }
}

table! {
    orders {
        id -> Integer,
        order_id -> Text,
        position -> Text,
        created_at -> BigInt,
    }
}

table! {
    btc_dai_orders {
        id -> Integer,
        order_id -> Integer,
        quantity -> Text,
        price -> Text,
        open -> Text,
        closed -> Text,
        settling -> Text,
        failed -> Text,
        cancelled -> Text,
    }
}

table! {
    order_hbit_params {
        id -> Integer,
        order_id -> Integer,
        network -> Text,
        side -> Text,
        our_final_address -> Text,
        expiry_offset -> BigInt,
    }
}

table! {
    order_herc20_params {
        id -> Integer,
        order_id -> Integer,
        chain_id -> BigInt,
        side -> Text,
        our_htlc_identity -> Text,
        token_contract -> Text,
        expiry_offset -> BigInt,
    }
}

table! {
    order_swaps {
        id -> Integer,
        order_id -> Integer,
        swap_id -> Integer,
    }
}

table! {
    completed_swaps {
        id -> Integer,
        swap_id -> Integer,
        completed_on -> BigInt,
    }
}

allow_tables_to_appear_in_same_query!(swaps, herc20s);
allow_tables_to_appear_in_same_query!(swaps, hbits);
allow_tables_to_appear_in_same_query!(hbits, herc20s);
allow_tables_to_appear_in_same_query!(orders, btc_dai_orders);
allow_tables_to_appear_in_same_query!(orders, order_hbit_params);
allow_tables_to_appear_in_same_query!(orders, order_herc20_params);
allow_tables_to_appear_in_same_query!(btc_dai_orders, order_hbit_params);
allow_tables_to_appear_in_same_query!(btc_dai_orders, order_herc20_params);
allow_tables_to_appear_in_same_query!(order_hbit_params, order_herc20_params);
allow_tables_to_appear_in_same_query!(orders, order_swaps);
allow_tables_to_appear_in_same_query!(swaps, order_swaps);
allow_tables_to_appear_in_same_query!(orders, swaps);
allow_tables_to_appear_in_same_query!(btc_dai_orders, swaps);
allow_tables_to_appear_in_same_query!(btc_dai_orders, order_swaps);
allow_tables_to_appear_in_same_query!(swap_contexts, swaps);
allow_tables_to_appear_in_same_query!(completed_swaps, swaps);
allow_tables_to_appear_in_same_query!(completed_swaps, swap_contexts);
joinable!(btc_dai_orders -> orders (order_id));
joinable!(order_hbit_params -> orders (order_id));
joinable!(order_herc20_params -> orders (order_id));
joinable!(order_swaps -> orders (order_id));
joinable!(order_swaps -> swaps (swap_id));
joinable!(completed_swaps -> swaps (swap_id));
joinable!(hbits -> swaps (swap_id));
