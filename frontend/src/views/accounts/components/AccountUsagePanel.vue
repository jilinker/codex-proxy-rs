<script setup lang="ts">
import type { AccountRow } from '../constants'
import { ChartNoAxesCombined } from '@lucide/vue'
import { ref } from 'vue'
import BaseIconButton from '@/components/base/BaseIconButton.vue'
import AccountQuotaForecastModal from './AccountQuotaForecastModal/index.vue'
import AccountUsageDetails from './AccountUsageDetails.vue'

defineProps<{ account: AccountRow }>()
const emit = defineEmits<{ accountUpdated: [account: AccountRow] }>()
const forecastOpen = ref(false)
</script>

<template>
  <AccountUsageDetails :usage="account.usage" show-billing>
    <template #actions>
      <BaseIconButton
        v-if="account.authenticationKind !== 'api_key'"
        label="预测周/月额度"
        size="sm"
        class="h-5 w-5"
        aria-haspopup="dialog"
        @click="forecastOpen = true"
      >
        <ChartNoAxesCombined class="size-3.5" :stroke-width="1.75" />
      </BaseIconButton>
    </template>
  </AccountUsageDetails>
  <AccountQuotaForecastModal
    v-if="account.authenticationKind !== 'api_key'"
    v-model="forecastOpen"
    :account="account"
    @account-updated="emit('accountUpdated', $event)"
  />
</template>
