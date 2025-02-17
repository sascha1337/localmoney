<script setup lang="ts">
import {
  calculateFiatPriceByRate,
  convertOfferRateToMarginRate,
  formatAmount,
} from '~/shared'
import { usePriceStore } from '~/stores/price'
import type { GetOffer } from '~/types/components.interface'
import { microDenomToDenom } from '~/utils/denom'

const props = defineProps<{ offer: GetOffer }>()
const priceStore = usePriceStore()
const marginRate = computed(() => convertOfferRateToMarginRate(props.offer.rate))
const usdRate = computed(() => priceStore.getPrice(props.offer.fiat_currency))
const offerPrice = computed(() => {
  return `${props.offer.fiat_currency} ${formatAmount(
      calculateFiatPriceByRate(usdRate.value, props.offer.rate),
      false,
  )}`
})
</script>

<template>
  <div :key="`${offer.id}-collapsed`" class="offer collapsed">
    <div class="offer-type">
      <p class="state">
        {{ offer.state }}
      </p>
      <p class="type">
        {{ offer.offer_type }}ing
      </p>
    </div>

    <div class="info">
      <!-- <p class="state">{{ offer.state }}</p>
      <div class="divider"></div> -->
      <div class="wrap">
        <p class="label">
          Limits
        </p>
        <p class="limit">
          {{ formatAmount(offer.min_amount) }} - {{ formatAmount(offer.max_amount) }}
          {{ microDenomToDenom(offer.denom.native) }}
        </p>
      </div>
      <div class="divider" />
      <div class="wrap">
        <p class="label">
          Price
        </p>
        <p class="rate">
          {{ marginRate.marginOffset }}%
          {{ marginRate.margin }} market
        </p>
      </div>
    </div>

    <div class="price">
      <p class="value">
        {{ offerPrice }}
      </p>
      <button class="secondary bg-gray300" type="button" @click="$emit('select')">
        edit
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
@import "../../style/tokens.scss";

.collapsed {
  .offer-type {
    display: flex;
    align-items: center;

    .state {
      margin-right: 24px;
      padding: 8px 16px;
      background-color: $gray150;
      border-radius: 8px;
      font-size: 14px;
      text-transform: capitalize;
      color: $gray900;
    }

    .type {
      font-size: 18px;
      font-weight: $semi-bold;
      color: $base-text;
      text-transform: capitalize;
    }
  }
}
</style>
