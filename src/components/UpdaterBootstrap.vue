<script setup lang="ts">
import { onMounted } from "vue";
import { useUpdater } from "../composables/useUpdater";

const { checkOnStartup } = useUpdater();

onMounted(checkOnStartup);
</script>

<template></template>
