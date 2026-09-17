#pragma once

#include <QByteArray>
#include <QDBusMessage>
#include <QHash>
#include <QJSValue>
#include <QNetworkAccessManager>
#include <QObject>
#include <QString>
#include <QVariant>

#include <functional>

class QQmlEngine;
class QDBusPendingCallWatcher;

// In-process network backend for the shell.
//  - HTTP: weather fetch via QNetworkAccessManager — zero forks, no curl.
//  - WiFi: NetworkManager D-Bus (org.freedesktop.NetworkManager) — zero
//    forks, no nmcli.
//  - Bluetooth: BlueZ D-Bus (org.bluez) — zero forks, no bluetoothctl.
// Every operation is async (QNetworkReply / QDBusPendingCallWatcher) and
// reports through a JS callback or a NOTIFY signal. Import as `import QsNet 1.0`.
class QsNet : public QObject {
	Q_OBJECT
public:
	explicit QsNet(QQmlEngine* engine, QObject* parent = nullptr);

	// ---- HTTP (weather; kills curl) --------------------------------------
	// GET `url`; timeoutMs in ms (0 = default). callback({ ok, body, error }).
	Q_INVOKABLE void httpGet(const QString& url, int timeoutMs = 10000, const QJSValue& callback = QJSValue());

	// ---- WiFi (NetworkManager D-Bus; kills nmcli) ------------------------
	Q_PROPERTY(bool wifiEnabled READ wifiEnabled NOTIFY wifiEnabledChanged)
	Q_PROPERTY(bool wifiScanning READ wifiScanning NOTIFY wifiScanningChanged)
	bool wifiEnabled() const { return m_wifiEnabled; }
	bool wifiScanning() const { return m_wifiScanning; }

	// Re-read WirelessEnabled from NetworkManager.
	Q_INVOKABLE void refreshWifiEnabled();
	// Flip WirelessEnabled (the NOTIFY signals fire when the daemon confirms).
	Q_INVOKABLE void toggleWifi();
	// Request a scan, then list access points. callback([ {name, signal,
	// secured, saved, active} ]). Callback fires ~1.6s after the scan request.
	Q_INVOKABLE void scanWifi(const QJSValue& callback = QJSValue());
	// Lightweight active-network read WITHOUT a rescan (pill status strip).
	// callback({ name, signal }) — empty object when nothing is connected.
	Q_INVOKABLE void wifiSignal(const QJSValue& callback = QJSValue());
	// Connect. Empty password = use a saved profile (NM picks it by SSID) or
	// create an open connection. callback({ ok, error }).
	Q_INVOKABLE void connectWifi(const QString& ssid, const QString& password = QString(), const QJSValue& callback = QJSValue());
	// Disconnect every active connection on the wifi device.
	Q_INVOKABLE void disconnectWifi(const QJSValue& callback = QJSValue());

	// ---- Bluetooth (BlueZ D-Bus; kills bluetoothctl) ---------------------
	Q_PROPERTY(bool bluetoothEnabled READ bluetoothEnabled NOTIFY bluetoothEnabledChanged)
	bool bluetoothEnabled() const { return m_btEnabled; }

	Q_INVOKABLE void refreshBluetoothEnabled();
	Q_INVOKABLE void toggleBluetooth();
	// List known devices. callback([ {address, name, connected} ]).
	Q_INVOKABLE void scanBluetooth(const QJSValue& callback = QJSValue());
	// Name of the currently connected device ("" when none). Lightweight.
	Q_INVOKABLE void btConnectedName(const QJSValue& callback = QJSValue());
	Q_INVOKABLE void connectBluetooth(const QString& address, const QJSValue& callback = QJSValue());
	Q_INVOKABLE void disconnectBluetooth(const QString& address, const QJSValue& callback = QJSValue());

public slots:
	// D-Bus PropertiesChanged handlers — keep the local mirrors truthful even
	// when state changes outside this shell (nm-applet, bluetoothctl, hw keys).
	void onNmPropsChanged(const QDBusMessage& message);
	void onBtPropsChanged(const QDBusMessage& message);

signals:
	void wifiEnabledChanged();
	void wifiScanningChanged();
	void bluetoothEnabledChanged();

private:
	// Generic async D-Bus method call → callback(message).
	void callDBus(const QString& svc, const QString& path, const QString& iface,
	              const QString& method, const QVariantList& args,
	              std::function<void(const QDBusMessage&)> cb);
	// Async property read → callback(value, ok).
	void readProp(const QString& svc, const QString& path, const QString& iface,
	              const QString& prop, std::function<void(const QVariant&, bool)> cb);
	// Resolve the wifi device path → callback(path, ok).
	void findWifiDevice(std::function<void(const QString&, bool)> cb);
	// Resolve the BlueZ adapter path → callback(path, ok).
	void findBtAdapter(std::function<void(const QString&, bool)> cb);
	// Build AddAndActivateConnection settings dict (empty password → saved/open).
	QVariantMap wifiSettings(const QString& ssid, const QString& password) const;
	// BlueZ device path for a MAC (dev_XX_XX_XX_XX_XX_XX).
	static QString btDevicePath(const QString& address);

	QQmlEngine* m_engine = nullptr;
	QNetworkAccessManager m_nam;
	bool m_wifiEnabled = false;
	bool m_wifiScanning = false;
	bool m_btEnabled = false;
	QString m_btAdapterPath;  // cached BlueZ adapter path (signal subscription)
	bool m_btSubscribed = false;
};
