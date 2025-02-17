<script setup lang="ts">
import { usePriceStore } from '~/stores/price'
import { OfferType } from '~/types/components.interface'
import type { GetOffer } from '~/types/components.interface'
import { calculateFiatPriceByRate, formatAmount, formatDate } from '~/shared'
import { useClientStore } from '~/stores/client'
import { microDenomToDenom } from '~/utils/denom'

const props = defineProps<{ offer: GetOffer }>()

const priceStore = usePriceStore()
const client = useClientStore()
const usdRate = computed(() => priceStore.getPrice(props.offer.fiat_currency))
const currentDate = computed(() => formatDate(new Date(props.offer.timestamp * 1000), false))
const fiatCurrency = computed(() => props.offer.fiat_currency)
const price = computed(() => {
  return `${props.offer.fiat_currency} ${formatAmount(
    calculateFiatPriceByRate(usdRate.value, props.offer.rate),
    false,
  )}`
})
const limit = computed(() => {
  const min = formatAmount(props.offer.min_amount)
  const max = formatAmount(props.offer.max_amount)
  const denom = microDenomToDenom(props.offer.denom.native)
  return `${min} - ${max} ${denom}`
})
const type = computed(() => props.offer.offer_type === OfferType.buy ? 'Buying' : 'Selling')

function unarchive() {
  client.unarchiveOffer({ ...props.offer })
}

onMounted(() => {
  priceStore.fetchPrices()
})
</script>

<template>
  <div class="wrap-table-item">
    <div class="col-1">
      <p>{{ currentDate }}</p>
    </div>
    <div class="col-2">
      <p>{{ type }}</p>
    </div>
    <div class="col-3">
      <p>{{ fiatCurrency }}</p>
    </div>
    <div class="col-4">
      <p>{{ limit }}</p>
    </div>
    <div class="col-5">
      <p>{{ price }}</p>
    </div>
    <div class="col-6 unarchive">
      <p @click="unarchive()">
        Unarchive
      </p>
    </div>
  </div>
</template>

<style lang="scss" scoped>
@import "../../style/tokens";

.wrap-table-item {
  display: flex;
  flex-direction: row;
  padding: 16px;

  p {
    font-size: 14px;
    font-weight: $regular;
  }

  .unarchive {
    cursor: pointer;
    color: $primary;

    p {
      font-weight: 600;
    }
  }
}
</style>
