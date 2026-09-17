#include "qsnet.hpp"

#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusObjectPath>
#include <QDBusPendingCall>
#include <QDBusPendingCallWatcher>
#include <QDBusVariant>
#include <QDebug>
#include <QJSEngine>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QQmlEngine>
#include <QQmlExtensionPlugin>
#include <QTimer>
#include <QUuid>
#include <qqml.h>

// ---------------------------------------------------------------------------
// QsNet — in-process network backend (HTTP + NetworkManager + BlueZ D-Bus).
// Kills curl (weather), nmcli (wifi) and bluetoothctl (bt). Every public call
// is async and reports through a QJSValue callback; state mirrors refresh via
// D-Bus PropertiesChanged subscriptions so the shell stays truthful even when
// state changes outside it (nm-applet, bluetoothctl, hardware switches).
// ---------------------------------------------------------------------------

// Snapshot of the NetworkManager object tree we care about, extracted from a
// single GetManagedObjects reply (one round trip, no polling).
struct NmState {
	QString wifiDevPath;      // Device.Wireless object path
	QString activeApPath;     // ActiveAccessPoint path ("" when disconnected)
	QString activeConnPath;   // Device.ActiveConnection path ("" when none)
	QMap<QString, QString> apSsid;    // AP path -> ssid
	QMap<QString, int> apSignal;      // AP path -> signal %
	QMap<QString, bool> apSecured;    // AP path -> wpa/rsn/wep present
	QMap<QString, QString> connUuid;  // Connection.Active path -> uuid
	QMap<QString, QString> connBySsid;  // wifi connection Id (== SSID) -> path
};

static bool parseNmObjects(const QDBusMessage& message, NmState& state) {
	if (message.type() != QDBusMessage::ReplyMessage)
		return false;

	QMap<QDBusObjectPath, QMap<QString, QVariantMap>> all;
	QDBusArgument arg = message.arguments().value(0).value<QDBusArgument>();
	arg >> all;

	for (auto it = all.constBegin(); it != all.constEnd(); ++it) {
		const QString path = it.key().path();
		const QMap<QString, QVariantMap>& ifaces = it.value();

		auto wdev = ifaces.constFind(QLatin1String("org.freedesktop.NetworkManager.Device.Wireless"));
		if (wdev != ifaces.constEnd()) {
			state.wifiDevPath = path;
			state.activeApPath = wdev->value(QLatin1String("ActiveAccessPoint")).value<QDBusObjectPath>().path();
			// The wifi device object ALSO exposes Device.ActiveConnection on this
			// same path — read it here so a non-wifi device (ethernet, lo)
			// iterated later can't overwrite the wifi connection path.
			auto devIface = ifaces.constFind(QLatin1String("org.freedesktop.NetworkManager.Device"));
			if (devIface != ifaces.constEnd())
				state.activeConnPath = devIface->value(QLatin1String("ActiveConnection")).value<QDBusObjectPath>().path();
		}

		auto ap = ifaces.constFind(QLatin1String("org.freedesktop.NetworkManager.AccessPoint"));
		if (ap != ifaces.constEnd()) {
			const QByteArray ssid = ap->value(QLatin1String("Ssid")).toByteArray();
			if (ssid.isEmpty())
				continue;
			state.apSsid[path] = QString::fromUtf8(ssid);
			state.apSignal[path] = ap->value(QLatin1String("Signal")).toUInt();
			const uint flags = ap->value(QLatin1String("Flags")).toUInt();
			const uint wpa = ap->value(QLatin1String("WpaFlags")).toUInt();
			const uint rsn = ap->value(QLatin1String("RsnFlags")).toUInt();
			state.apSecured[path] = (wpa != 0) || (rsn != 0) || ((flags & 0x1u) != 0);
		}

		auto conn = ifaces.constFind(QLatin1String("org.freedesktop.NetworkManager.Connection.Active"));
		if (conn != ifaces.constEnd())
			state.connUuid[path] = conn->value(QLatin1String("Uuid")).toString();

		auto sconn = ifaces.constFind(QLatin1String("org.freedesktop.NetworkManager.Settings.Connection"));
		if (sconn != ifaces.constEnd()) {
			const QVariantMap& sp = sconn.value();
			if (sp.value(QLatin1String("Type")).toString() == QLatin1String("802-11-wireless"))
				state.connBySsid[sp.value(QLatin1String("Id")).toString()] = path;
		}
	}
	return !state.wifiDevPath.isEmpty();
}

QsNet::QsNet(QQmlEngine* engine, QObject* parent)
	: QObject(parent), m_engine(engine) {
	QDBusConnection bus = QDBusConnection::systemBus();
	bus.connect(QLatin1String("org.freedesktop.NetworkManager"),
	            QLatin1String("/org/freedesktop/NetworkManager"),
	            QLatin1String("org.freedesktop.DBus.Properties"),
	            QLatin1String("PropertiesChanged"),
	            this, SLOT(onNmPropsChanged(QDBusMessage)));
}

void QsNet::onNmPropsChanged(const QDBusMessage& message) {
	if (message.arguments().size() < 2)
		return;
	const QVariantMap changed = message.arguments().at(1).value<QVariantMap>();
	if (changed.contains(QLatin1String("WirelessEnabled"))) {
		m_wifiEnabled = changed.value(QLatin1String("WirelessEnabled")).toBool();
		emit wifiEnabledChanged();
	}
}

void QsNet::onBtPropsChanged(const QDBusMessage& message) {
	if (message.arguments().size() < 2)
		return;
	const QVariantMap changed = message.arguments().at(1).value<QVariantMap>();
	if (changed.contains(QLatin1String("Powered"))) {
		m_btEnabled = changed.value(QLatin1String("Powered")).toBool();
		emit bluetoothEnabledChanged();
	}
}

// ---- generic async helpers ------------------------------------------------

void QsNet::callDBus(const QString& svc, const QString& path, const QString& iface,
                     const QString& method, const QVariantList& args,
                     std::function<void(const QDBusMessage&)> cb) {
	QDBusMessage msg = QDBusMessage::createMethodCall(svc, path, iface, method);
	msg.setArguments(args);
	QDBusPendingCall call = QDBusConnection::systemBus().asyncCall(msg);
	QDBusPendingCallWatcher* watcher = new QDBusPendingCallWatcher(call, this);
	connect(watcher, &QDBusPendingCallWatcher::finished, this, [watcher, cb]() {
		const QDBusMessage reply = watcher->reply();
		watcher->deleteLater();
		cb(reply);
	});
}

void QsNet::readProp(const QString& svc, const QString& path, const QString& iface,
                     const QString& prop, std::function<void(const QVariant&, bool)> cb) {
	callDBus(svc, path, QLatin1String("org.freedesktop.DBus.Properties"), QLatin1String("Get"),
	         {iface, prop}, [cb](const QDBusMessage& m) {
		if (m.type() != QDBusMessage::ReplyMessage) {
			cb(QVariant(), false);
			return;
		}
		cb(m.arguments().value(0).value<QDBusVariant>().variant(), true);
	});
}

// ---- HTTP (weather) -------------------------------------------------------

void QsNet::httpGet(const QString& url, int timeoutMs, const QJSValue& callback) {
	QNetworkRequest request(url);
	request.setTransferTimeout(timeoutMs > 0 ? timeoutMs : 10000);
	QNetworkReply* reply = m_nam.get(request);
	connect(reply, &QNetworkReply::finished, this, [this, reply, callback]() {
		const QNetworkReply::NetworkError err = reply->error();
		QJSValue result = m_engine->newObject();
		result.setProperty(QLatin1String("ok"), err == QNetworkReply::NoError);
		result.setProperty(QLatin1String("error"), reply->errorString());
		result.setProperty(QLatin1String("body"), QString::fromUtf8(reply->readAll()));
		reply->deleteLater();
		if (callback.isCallable())
			callback.call({result});
	});
}

// ---- WiFi (NetworkManager) ------------------------------------------------

void QsNet::refreshWifiEnabled() {
	readProp(QLatin1String("org.freedesktop.NetworkManager"),
	         QLatin1String("/org/freedesktop/NetworkManager"),
	         QLatin1String("org.freedesktop.NetworkManager"),
	         QLatin1String("WirelessEnabled"),
	         [this](const QVariant& v, bool ok) {
		if (!ok)
			return;
		const bool en = v.toBool();
		if (en != m_wifiEnabled) {
			m_wifiEnabled = en;
			emit wifiEnabledChanged();
		}
	});
}

void QsNet::toggleWifi() {
	const bool target = !m_wifiEnabled;
	callDBus(QLatin1String("org.freedesktop.NetworkManager"),
	         QLatin1String("/org/freedesktop/NetworkManager"),
	         QLatin1String("org.freedesktop.DBus.Properties"),
	         QLatin1String("Set"),
	         {QLatin1String("org.freedesktop.NetworkManager"), QLatin1String("WirelessEnabled"),
	          QVariant::fromValue(QDBusVariant(target))},
	         [this](const QDBusMessage&) { refreshWifiEnabled(); });
}

void QsNet::findWifiDevice(std::function<void(const QString&, bool)> cb) {
	callDBus(QLatin1String("org.freedesktop.NetworkManager"),
	         QLatin1String("/org/freedesktop/NetworkManager"),
	         QLatin1String("org.freedesktop.DBus.ObjectManager"),
	         QLatin1String("GetManagedObjects"), {},
	         [cb](const QDBusMessage& m) {
		NmState st;
		if (!parseNmObjects(m, st)) {
			cb(QString(), false);
			return;
		}
		cb(st.wifiDevPath, true);
	});
}

void QsNet::scanWifi(const QJSValue& callback) {
	const auto fire = [this, callback](const QJSValue& list) {
		m_wifiScanning = false;
		emit wifiScanningChanged();
		if (callback.isCallable())
			callback.call({list});
	};

	if (!m_wifiEnabled) {
		fire(m_engine->newArray(0));
		return;
	}

	m_wifiScanning = true;
	emit wifiScanningChanged();

	findWifiDevice([this, fire](const QString& devPath, bool ok) {
		if (!ok || devPath.isEmpty()) {
			fire(m_engine->newArray(0));
			return;
		}
		callDBus(QLatin1String("org.freedesktop.NetworkManager"), devPath,
		         QLatin1String("org.freedesktop.NetworkManager.Device.Wireless"),
		         QLatin1String("RequestScan"), {QVariant(QVariantMap())},
		         [](const QDBusMessage&) {});
		// NM applies scans asynchronously; give it a moment, then read the tree.
		QTimer::singleShot(1600, this, [this, fire]() {
			callDBus(QLatin1String("org.freedesktop.NetworkManager"),
			         QLatin1String("/org/freedesktop/NetworkManager"),
			         QLatin1String("org.freedesktop.DBus.ObjectManager"),
			         QLatin1String("GetManagedObjects"), {},
			         [this, fire](const QDBusMessage& m) {
				NmState st;
				if (!parseNmObjects(m, st)) {
					fire(m_engine->newArray(0));
					return;
				}
				QJSValue arr = m_engine->newArray();
				int n = 0;
				for (auto it = st.apSsid.constBegin(); it != st.apSsid.constEnd(); ++it) {
					QJSValue o = m_engine->newObject();
					o.setProperty(QLatin1String("name"), it.value());
					o.setProperty(QLatin1String("signal"), st.apSignal.value(it.key()));
					o.setProperty(QLatin1String("secured"), st.apSecured.value(it.key()));
					o.setProperty(QLatin1String("active"), it.key() == st.activeApPath);
					arr.setProperty(n++, o);
				}
				fire(arr);
			});
		});
	});
}

void QsNet::wifiSignal(const QJSValue& callback) {
	callDBus(QLatin1String("org.freedesktop.NetworkManager"),
	         QLatin1String("/org/freedesktop/NetworkManager"),
	         QLatin1String("org.freedesktop.DBus.ObjectManager"),
	         QLatin1String("GetManagedObjects"), {},
	         [this, callback](const QDBusMessage& m) {
		QJSValue res = m_engine->newObject();
		NmState st;
		if (parseNmObjects(m, st) && !st.activeApPath.isEmpty()) {
			res.setProperty(QLatin1String("name"), st.apSsid.value(st.activeApPath));
			res.setProperty(QLatin1String("signal"), st.apSignal.value(st.activeApPath));
			res.setProperty(QLatin1String("uuid"), st.connUuid.value(st.activeConnPath));
		} else {
			res.setProperty(QLatin1String("name"), QString());
			res.setProperty(QLatin1String("signal"), -1);
			res.setProperty(QLatin1String("uuid"), QString());
		}
		if (callback.isCallable())
			callback.call({res});
	});
}

QVariantMap QsNet::wifiSettings(const QString& ssid, const QString& password) const {
	QVariantMap con;
	con.insert(QLatin1String("type"), QLatin1String("802-11-wireless"));
	con.insert(QLatin1String("id"), ssid);
	con.insert(QLatin1String("uuid"), QUuid::createUuid().toString(QUuid::WithoutBraces));

	QVariantMap w;
	w.insert(QLatin1String("ssid"), ssid.toUtf8());
	w.insert(QLatin1String("mode"), QLatin1String("infrastructure"));

	QVariantMap ip4;
	ip4.insert(QLatin1String("method"), QLatin1String("auto"));
	QVariantMap ip6;
	ip6.insert(QLatin1String("method"), QLatin1String("auto"));

	QVariantMap top;
	top.insert(QLatin1String("connection"), con);
	top.insert(QLatin1String("802-11-wireless"), w);
	top.insert(QLatin1String("ipv4"), ip4);
	top.insert(QLatin1String("ipv6"), ip6);
	if (!password.isEmpty()) {
		QVariantMap sec;
		sec.insert(QLatin1String("key-mgmt"), QLatin1String("wpa-psk"));
		sec.insert(QLatin1String("psk"), password);
		top.insert(QLatin1String("802-11-wireless-security"), sec);
	}
	return top;
}

void QsNet::connectWifi(const QString& ssid, const QString& password, const QJSValue& callback) {
	// One GetManagedObjects gives us both the wifi device and the saved
	// profile table — no extra round trips.
	callDBus(QLatin1String("org.freedesktop.NetworkManager"),
	         QLatin1String("/org/freedesktop/NetworkManager"),
	         QLatin1String("org.freedesktop.DBus.ObjectManager"),
	         QLatin1String("GetManagedObjects"), {},
	         [this, ssid, password, callback](const QDBusMessage& m) {
		const auto fireResult = [this, callback](bool ok, const QString& error) {
			QJSValue res = m_engine->newObject();
			res.setProperty(QLatin1String("ok"), ok);
			res.setProperty(QLatin1String("error"), error);
			if (callback.isCallable())
				callback.call({res});
		};

		NmState st;
		if (!parseNmObjects(m, st) || st.wifiDevPath.isEmpty()) {
			fireResult(false, QStringLiteral("No wifi device found"));
			return;
		}
		const QString devPath = st.wifiDevPath;

		// Empty password → reuse the saved profile for this SSID when one
		// exists (matches `nmcli dev wifi connect <ssid>` semantics); only
		// create a fresh connection when there is nothing to reuse.
		if (password.isEmpty()) {
			auto saved = st.connBySsid.constFind(ssid);
			if (saved != st.connBySsid.constEnd()) {
				callDBus(QLatin1String("org.freedesktop.NetworkManager"),
				         QLatin1String("/org/freedesktop/NetworkManager"),
				         QLatin1String("org.freedesktop.NetworkManager"),
				         QLatin1String("ActivateConnection"),
				         {QVariant::fromValue(QDBusObjectPath(saved.value())),
				          QVariant::fromValue(QDBusObjectPath(devPath)),
				          QVariant::fromValue(QDBusObjectPath(QLatin1String("/")))},
				         [this, fireResult](const QDBusMessage& rm) {
					fireResult(rm.type() == QDBusMessage::ReplyMessage, rm.errorMessage());
				});
				return;
			}
		}

		callDBus(QLatin1String("org.freedesktop.NetworkManager"),
		         QLatin1String("/org/freedesktop/NetworkManager"),
		         QLatin1String("org.freedesktop.NetworkManager"),
		         QLatin1String("AddAndActivateConnection"),
		         {QVariant::fromValue(wifiSettings(ssid, password)),
		          QVariant::fromValue(QDBusObjectPath(devPath)),
		          QVariant::fromValue(QDBusObjectPath(QLatin1String("/")))},
		         [this, fireResult](const QDBusMessage& am) {
			fireResult(am.type() == QDBusMessage::ReplyMessage, am.errorMessage());
		});
	});
}

void QsNet::disconnectWifi(const QJSValue& callback) {
	findWifiDevice([this, callback](const QString& devPath, bool ok) {
		if (!ok || devPath.isEmpty()) {
			if (callback.isCallable()) {
				QJSValue res = m_engine->newObject();
				res.setProperty(QLatin1String("ok"), true);
				callback.call({res});
			}
			return;
		}
		readProp(QLatin1String("org.freedesktop.NetworkManager"), devPath,
		         QLatin1String("org.freedesktop.NetworkManager.Device"),
		         QLatin1String("ActiveConnection"),
		         [this, callback](const QVariant& v, bool ok) {
			const QString connPath = v.value<QDBusObjectPath>().path();
			if (!ok || connPath.isEmpty() || connPath == QLatin1String("/")) {
				if (callback.isCallable()) {
					QJSValue res = m_engine->newObject();
					res.setProperty(QLatin1String("ok"), true);
					callback.call({res});
				}
				return;
			}
			callDBus(QLatin1String("org.freedesktop.NetworkManager"),
			         QLatin1String("/org/freedesktop/NetworkManager"),
			         QLatin1String("org.freedesktop.NetworkManager"),
			         QLatin1String("DeactivateConnection"),
			         {QVariant::fromValue(QDBusObjectPath(connPath))},
			         [this, callback](const QDBusMessage& m) {
				if (callback.isCallable()) {
					QJSValue res = m_engine->newObject();
					res.setProperty(QLatin1String("ok"), m.type() == QDBusMessage::ReplyMessage);
					res.setProperty(QLatin1String("error"), m.errorMessage());
					callback.call({res});
				}
			});
		});
	});
}

// ---- Bluetooth (BlueZ) ----------------------------------------------------

QString QsNet::btDevicePath(const QString& address) {
	QString norm = address.toLower();
	norm.replace(QLatin1Char(':'), QLatin1Char('_'));
	return QStringLiteral("/org/bluez/dev_") + norm;
}

void QsNet::findBtAdapter(std::function<void(const QString&, bool)> cb) {
	callDBus(QLatin1String("org.bluez"), QLatin1String("/"),
	         QLatin1String("org.freedesktop.DBus.ObjectManager"),
	         QLatin1String("GetManagedObjects"), {},
	         [cb](const QDBusMessage& m) {
		if (m.type() != QDBusMessage::ReplyMessage) {
			cb(QString(), false);
			return;
		}
		QMap<QDBusObjectPath, QMap<QString, QVariantMap>> all;
		QDBusArgument arg = m.arguments().value(0).value<QDBusArgument>();
		arg >> all;
		for (auto it = all.constBegin(); it != all.constEnd(); ++it) {
			if (it.value().contains(QLatin1String("org.bluez.Adapter1"))) {
				cb(it.key().path(), true);
				return;
			}
		}
		cb(QString(), false);
	});
}

void QsNet::refreshBluetoothEnabled() {
	findBtAdapter([this](const QString& path, bool ok) {
		if (!ok || path.isEmpty())
			return;
		if (path != m_btAdapterPath) {
			QDBusConnection bus = QDBusConnection::systemBus();
			if (m_btSubscribed)
				bus.disconnect(QLatin1String("org.bluez"), m_btAdapterPath,
				               QLatin1String("org.freedesktop.DBus.Properties"),
				               QLatin1String("PropertiesChanged"), this,
				               SLOT(onBtPropsChanged(QDBusMessage)));
			m_btSubscribed = true;
			m_btAdapterPath = path;
			bus.connect(QLatin1String("org.bluez"), path,
			            QLatin1String("org.freedesktop.DBus.Properties"),
			            QLatin1String("PropertiesChanged"), this,
			            SLOT(onBtPropsChanged(QDBusMessage)));
		}
		readProp(QLatin1String("org.bluez"), path, QLatin1String("org.bluez.Adapter1"),
		         QLatin1String("Powered"), [this](const QVariant& v, bool ok) {
			if (!ok)
				return;
			const bool en = v.toBool();
			if (en != m_btEnabled) {
				m_btEnabled = en;
				emit bluetoothEnabledChanged();
			}
		});
	});
}

void QsNet::toggleBluetooth() {
	findBtAdapter([this](const QString& path, bool ok) {
		if (!ok || path.isEmpty())
			return;
		const bool target = !m_btEnabled;
		callDBus(QLatin1String("org.bluez"), path, QLatin1String("org.freedesktop.DBus.Properties"),
		         QLatin1String("Set"),
		         {QLatin1String("org.bluez.Adapter1"), QLatin1String("Powered"),
		          QVariant::fromValue(QDBusVariant(target))},
		         [this](const QDBusMessage&) { refreshBluetoothEnabled(); });
	});
}

void QsNet::scanBluetooth(const QJSValue& callback) {
	callDBus(QLatin1String("org.bluez"), QLatin1String("/"),
	         QLatin1String("org.freedesktop.DBus.ObjectManager"),
	         QLatin1String("GetManagedObjects"), {},
	         [this, callback](const QDBusMessage& m) {
		QJSValue arr = m_engine->newArray();
		if (m.type() == QDBusMessage::ReplyMessage) {
			QMap<QDBusObjectPath, QMap<QString, QVariantMap>> all;
			QDBusArgument arg = m.arguments().value(0).value<QDBusArgument>();
			arg >> all;
			int n = 0;
			for (auto it = all.constBegin(); it != all.constEnd(); ++it) {
				auto ditem = it.value().constFind(QLatin1String("org.bluez.Device1"));
				if (ditem == it.value().constEnd())
					continue;
				const QVariantMap& p = ditem.value();
				QJSValue o = m_engine->newObject();
				o.setProperty(QLatin1String("address"), p.value(QLatin1String("Address")).toString());
				QString name = p.value(QLatin1String("Alias")).toString();
				if (name.isEmpty())
					name = p.value(QLatin1String("Name")).toString();
				o.setProperty(QLatin1String("name"), name);
				o.setProperty(QLatin1String("connected"), p.value(QLatin1String("Connected")).toBool());
				arr.setProperty(n++, o);
			}
		}
		if (callback.isCallable())
			callback.call({arr});
	});
}

void QsNet::btConnectedName(const QJSValue& callback) {
	callDBus(QLatin1String("org.bluez"), QLatin1String("/"),
	         QLatin1String("org.freedesktop.DBus.ObjectManager"),
	         QLatin1String("GetManagedObjects"), {},
	         [this, callback](const QDBusMessage& m) {
		QString name;
		if (m.type() == QDBusMessage::ReplyMessage) {
			QMap<QDBusObjectPath, QMap<QString, QVariantMap>> all;
			QDBusArgument arg = m.arguments().value(0).value<QDBusArgument>();
			arg >> all;
			for (auto it = all.constBegin(); it != all.constEnd(); ++it) {
				auto ditem = it.value().constFind(QLatin1String("org.bluez.Device1"));
				if (ditem == it.value().constEnd())
					continue;
				if (ditem->value(QLatin1String("Connected")).toBool()) {
					name = ditem->value(QLatin1String("Alias")).toString();
					if (name.isEmpty())
						name = ditem->value(QLatin1String("Name")).toString();
					break;
				}
			}
		}
		if (callback.isCallable())
			callback.call({QJSValue(name)});
	});
}

void QsNet::connectBluetooth(const QString& address, const QJSValue& callback) {
	callDBus(QLatin1String("org.bluez"), btDevicePath(address),
	         QLatin1String("org.bluez.Device1"), QLatin1String("Connect"), {},
	         [this, callback](const QDBusMessage& m) {
		QJSValue res = m_engine->newObject();
		res.setProperty(QLatin1String("ok"), m.type() == QDBusMessage::ReplyMessage);
		res.setProperty(QLatin1String("error"), m.errorMessage());
		if (callback.isCallable())
			callback.call({res});
	});
}

void QsNet::disconnectBluetooth(const QString& address, const QJSValue& callback) {
	callDBus(QLatin1String("org.bluez"), btDevicePath(address),
	         QLatin1String("org.bluez.Device1"), QLatin1String("Disconnect"), {},
	         [this, callback](const QDBusMessage& m) {
		QJSValue res = m_engine->newObject();
		res.setProperty(QLatin1String("ok"), m.type() == QDBusMessage::ReplyMessage);
		res.setProperty(QLatin1String("error"), m.errorMessage());
		if (callback.isCallable())
			callback.call({res});
	});
}

// Classic loadable QML plugin: registers QsNet as the "QsNet" singleton.
class QsNetPlugin : public QQmlExtensionPlugin {
	Q_OBJECT;
	Q_PLUGIN_METADATA(IID QQmlExtensionInterface_iid);

public:
	void registerTypes(const char* uri) override {
		qmlRegisterSingletonType<QsNet>(uri, 1, 0, "QsNet", [](QQmlEngine* engine, QJSEngine*) {
			return new QsNet(engine);
		});
	}
};

#include "qsnet.moc"
