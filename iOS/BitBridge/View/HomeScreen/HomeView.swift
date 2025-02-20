//
//  HomeView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 18/2/25.
//

import Foundation
import SwiftUI

struct HomeView: View {
    @StateObject private var core = Core()
    
    init() {
        UITabBar.appearance().unselectedItemTintColor = Theme.LightViolet.uiColor
            .withAlphaComponent(0.4)
    }
    
    var body: some View {
        TabView {
            ReceiveView()
                .tabItem {
                    ButtonNavigation(icon: "arrow.down.circle", label: "Receive")
                }
            ShareView()
                .tabItem {
                    ButtonNavigation(icon: "globe.europe.africa.fill", label: "Share")
                }
            SettingView()
                .tabItem {
                    ButtonNavigation(icon: "gearshape", label: "Settings")
                }
        }
        .accentColor(Theme.LightViolet.color)
    }
}

#Preview("HomeView") {
    HomeView()
}
