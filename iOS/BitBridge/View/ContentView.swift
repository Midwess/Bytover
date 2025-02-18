//
//  ContentView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SplineRuntime
import SwiftUI
import SharedTypes

struct ContentView: View {
    @StateObject private var core: Core = Core()
    var body: some View {
        LandingView()
            .navigate(to: HomeView(), when: $core.is_signed_in)
    }
}
struct CounterView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
