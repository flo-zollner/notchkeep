<script lang="ts">
  import AccountSettingsForm from '$lib/components/AccountSettingsForm.svelte';
  import Sheet from './Sheet.svelte';
  import { api, type Account } from '$lib/api';
  import { t } from '$lib/settings.svelte';

  // i18n-Lookup ohne typed Property-Access — fehlende Keys fallen via ?? auf den Default zurück.
  const tx = () => t().common as unknown as Record<string, string | undefined>;

  interface Props {
    account: Account;
    onClose: () => void;
    onSaved: (a: Account) => void;
  }
  let { account, onClose, onSaved }: Props = $props();

  async function handleSave(draft: Account) {
    await api.updateAccount(draft);
    onSaved(draft);
    onClose();
  }
</script>

<Sheet open={true} {onClose} title={tx().editAccount ?? 'Konto bearbeiten'}>
  <AccountSettingsForm {account} onSave={handleSave} onCancel={onClose} />
</Sheet>
