<script setup lang="ts">
// https://github.com/vueuse/head
// you can use this to manipulate the document head in any components,
// they will be rendered correctly in the html results with vite-ssg
import { ChainClient } from '~/network/Chain'
import { useClientStore } from '~/stores/client'
import './ui/style/reset.scss'

const client = useClientStore()
client.setClient(ChainClient.kujira) // required to properly init chain
const loading = computed(() => client.loadingState)

useHead({
  title: 'Local P2P - DEVNET',
  meta: [
    { name: 'description', content: 'Decentralize the ramps.' },
  ],
})
</script>

<template>
  <Header />
  <RouterView />
  <!-- Loading Modal -->
  <ModalLoading :loading="loading" />
</template>

<style lang="scss">
@import "./ui/style/tokens.scss";

/* Main Style */
body {
    margin: 0 auto;
    font-family: "Poppins", sans-serif;
    color: $base-text;
    background-color: $background;
    min-width: 1000px;

    @media only screen and (max-width: 550px) {
        min-width: 0px;
    }
}

button {
    height: 40px;
    border-radius: 8px;
    font-family: "Poppins", sans-serif;
    font-weight: 700;
    cursor: pointer;
}

::selection {
    color: $base-text;
    background-color: $primary;
}
</style>
