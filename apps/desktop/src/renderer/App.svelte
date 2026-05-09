<script>
  let status = 'Initializing...';
  
  async function checkHealth() {
    try {
      const result = await window.api.healthCheck();
      status = `Healthy: ${new Date(result.timestamp).toLocaleTimeString()}`;
    } catch (e) {
      status = 'Error: ' + e.message;
    }
  }
  
  checkHealth();
</script>

<main class="p-8">
  <h1 class="text-3xl font-bold mb-4">OpenWhisper</h1>
  <p class="text-gray-600 mb-4">Offline dictation for Windows</p>
  <div class="bg-gray-100 p-4 rounded">
    <p>Status: {status}</p>
  </div>
  <button 
    class="mt-4 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
    on:click={checkHealth}>
    Check Health
  </button>
</main>