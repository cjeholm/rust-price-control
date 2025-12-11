let chartRef = null;

async function priceChart() {
  const res = await fetch("/today");
  const data = await res.json();

  const res2 = await fetch("/config");
  const config = await res2.json();
  const currencyKey = config.currency;

  // --- FETCH TOMORROW (OPTIONAL) ---
  let tomorrow = [];
  try {
    const res2 = await fetch("/tomorrow");
    if (res2.ok) {
      const temp = await res2.json();
      if (Array.isArray(temp) && temp.length > 0) {
        tomorrow = temp;
      }
    }
  } catch {
    tomorrow = [];
  }

  // TODAY labels + values
  const labelsToday = data.map((item) => item.time_start.slice(11, 16));
  const valuesToday = data.map((item) => item[currencyKey]);

  // Compute current 15-min slot
  const now = new Date();
  const hours = now.getHours().toString().padStart(2, "0");
  const minutes = Math.floor(now.getMinutes() / 15) * 15;
  const mins = minutes.toString().padStart(2, "0");
  const currentLabel = `${hours}:${mins}`;

  // Base colors for today
  const baseColorsToday = labelsToday.map((l) =>
    l === currentLabel ? "orange" : "#888",
  );

  // Default merged labels and dataset
  let labels = [...labelsToday];
  let dataTodayMerged = [...valuesToday];
  let dataTomorrowMerged = null;

  // If tomorrow exists → merge into chart
  if (tomorrow.length > 0) {
    const labelsTomorrow = tomorrow.map((item) =>
      item.time_start.slice(11, 16),
    );
    const valuesTomorrow = tomorrow.map((item) => item[currencyKey]);

    // Add tomorrow labels after today
    labels = [...labelsToday, ...labelsTomorrow];

    // Align datasets: today data then nulls
    dataTodayMerged = [
      ...valuesToday,
      ...Array(valuesTomorrow.length).fill(null),
    ];

    // Tomorrow dataset starts with nulls then data
    dataTomorrowMerged = [
      ...Array(valuesToday.length).fill(null),
      ...valuesTomorrow,
    ];
  }

  const ctx = document.getElementById("priceChart");

  // Replace chart if already created
  if (chartRef) chartRef.destroy();

  // Build datasets array
  const datasets = [
    {
      label: `${currencyKey} (Today)`,
      data: dataTodayMerged,
      backgroundColor: [
        ...baseColorsToday,
        ...(dataTomorrowMerged
          ? Array(dataTomorrowMerged.length - valuesToday.length).fill("#888")
          : []),
      ],
      hoverBackgroundColor: "yellow", // bar turns yellow when hovered
    },
  ];

  if (dataTomorrowMerged) {
    datasets.push({
      label: `${currencyKey} (Tomorrow)`,
      data: dataTomorrowMerged,
      backgroundColor: "#555",
      hoverBackgroundColor: "yellow", // bar turns yellow when hovered
    });
  }

  chartRef = new Chart(ctx, {
    type: "bar",
    data: {
      labels,
      datasets,
    },
    options: {
      maintainAspectRatio: false,
      responsive: true,
      // animation: { duration: 200 },
      animation: false,
      plugins: {
        legend: {
          labels: {
            font: { size: 16 },
          },
        },
        tooltip: {
          bodyFont: { size: 16 },
          titleFont: { size: 16 },
        },
      },
      scales: {
        x: {
          ticks: {
            font: { size: 16 }, // timestamps
          },
        },
        y: {
          ticks: {
            font: { size: 16 }, // left-side numbers
          },
        },
      },
    },
  });
  // Find current price
  let currentPrice = null;
  const idx = labelsToday.indexOf(currentLabel);
  if (idx !== -1) {
    currentPrice = valuesToday[idx];
  }

  // Update the DOM
  const totalPrice =
    (currentPrice +
      config.grid_fee +
      config.energy_tax +
      config.variable_costs +
      config.spot_fee +
      config.cert_fee) *
    (1 + config.vat);

  const container = document.getElementById("prices");
  container.innerHTML = "";
  const spot = document.createElement("div");
  spot.className = "price-card";
  spot.innerHTML = `
            Spot price: <strong>${currentPrice.toFixed(4)}</strong> <br>
          `;
  container.appendChild(spot);
  const total = document.createElement("div");
  total.className = "price-card";
  total.innerHTML = `
            Total price: <strong>${totalPrice.toFixed(4)}</strong> <br>
          `;
  container.appendChild(total);
  deviceList();
}

async function deviceList() {
  const res = await fetch("/devices");
  const json = await res.json();
  const res2 = await fetch("/config");
  const config = await res2.json();

  const container = document.getElementById("devices");
  container.innerHTML = "";

  for (const d of json.device) {
    const card = document.createElement("div");
    card.className = "device-card";

    // Store price for hover
    card.dataset.today_trigger_price = d.today_trigger_price;
    card.dataset.tomorrow_trigger_price = d.tomorrow_trigger_price;

    let stateClass;
    if (d.state === "On") stateClass = "state-on";
    else if (d.state === "Off") stateClass = "state-off";
    else stateClass = "state-unknown";

    let html = "";

    if (d.mode === "Price") {
      html = `
    <span class="state ${stateClass}"><strong>${d.name}</strong></span><br>
    Mode: ${d.mode}<br>
    Telldus: ${d.telldus}<br>
    Price: ${d.price}<br>
    <s>Ratio: ${d.ratio}</s><br>
  `;
    } else if (d.mode === "Ratio") {
      html = `
    <span class="state ${stateClass}"><strong>${d.name}</strong></span><br>
    Mode: ${d.mode}<br>
    Telldus: ${d.telldus}<br>
    <s>Price: ${d.price}</s><br>
    Ratio: ${d.ratio}<br>
  `;
    } else {
      html = `
    <span class="state ${stateClass}"><strong>${d.name}</strong></span><br>
    Mode: ${d.mode}<br>
    <s>Telldus: ${d.telldus}</s><br>
    <s>Price: ${d.price}</s><br>
    <s>Ratio: ${d.ratio}</s><br>
  `;
    }

    if (config.webui_toggle === true) {
      html += `<button class="switch-on">On</button> <button class="switch-off">Off</button>`;
    }

    card.innerHTML = html;

    if (config.webui_toggle) {
      const btnOn = card.querySelector(".switch-on");
      const btnOff = card.querySelector(".switch-off");

      btnOn.addEventListener("click", async () => {
        try {
          await fetch(`/switchon/${encodeURIComponent(d.name)}`, {
            method: "POST",
          });
          console.log(`${d.name} switched on`);
        } catch (err) {
          console.error("Error switching on:", err);
        }
      });

      btnOff.addEventListener("click", async () => {
        try {
          await fetch(`/switchoff/${encodeURIComponent(d.name)}`, {
            method: "POST",
          });
          console.log(`${d.name} switched off`);
        } catch (err) {
          console.error("Error switching off:", err);
        }
      });
    }

    container.appendChild(card);
  }
}

document.addEventListener("mouseover", (e) => {
  const card = e.target.closest(".device-card");
  if (!card || !chartRef) return;

  const threshold_today = parseFloat(card.dataset.today_trigger_price);
  const threshold_tomorrow = parseFloat(card.dataset.tomorrow_trigger_price);

  // TODAY (dataset 0)
  const ds0 = chartRef.data.datasets[0];
  ds0.backgroundColor = ds0.data.map((v, i) => {
    if (chartRef.data.labels[i] === undefined) return "#444";
    if (ds0.backgroundColor[i] === "orange") return "orange";
    return v < threshold_today ? "green" : "#444";
  });

  // TOMORROW (dataset 1) only if present
  const ds1 = chartRef.data.datasets[1];
  if (ds1) {
    ds1.backgroundColor = ds1.data.map((v, i) =>
      v < threshold_tomorrow ? "green" : "#444",
    );
  }

  chartRef.update();
});

document.addEventListener("mouseout", (e) => {
  const card = e.target.closest(".device-card");
  if (!card || !chartRef) return;

  const labels = chartRef.data.labels;

  const now = new Date();
  const hours = now.getHours().toString().padStart(2, "0");
  const minutes = Math.floor(now.getMinutes() / 15) * 15;
  const mins = minutes.toString().padStart(2, "0");
  const currentLabel = `${hours}:${mins}`;

  // Reset TODAY
  chartRef.data.datasets[0].backgroundColor = labels.map((l) =>
    l === currentLabel ? "orange" : "#888",
  );

  // Reset TOMORROW only if present
  const ds1 = chartRef.data.datasets[1];
  if (ds1) {
    ds1.backgroundColor = labels.map(() => "#888");
  }

  chartRef.update();
});

async function checkBackendHealth() {
  const el = document.getElementById("health");
  if (!el) return;

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);

    const res = await fetch("/health", { signal: controller.signal });
    clearTimeout(timeout);

    if (res.ok) {
      el.textContent = "Connected ✔️";
      el.style.color = "green";
    } else {
      el.textContent = "Disconnected ❌";
      el.style.color = "red";
    }
  } catch {
    const el = document.getElementById("health");
    if (!el) return;
    el.textContent = "Disconnected ❌";
    el.style.color = "red";
  }
}

// Refresh price chart
setInterval(priceChart, 120 * 1000);
priceChart();

// Refresh device list
setInterval(deviceList, 5 * 1000);
deviceList();

// Check server health
setInterval(checkBackendHealth, 3 * 1000);
checkBackendHealth();
